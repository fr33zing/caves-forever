use std::{
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
    str::FromStr,
    time::SystemTime,
};

use anyhow::anyhow;
use bevy::prelude::*;
use lib::worldgen::asset::{Room, Tunnel};
use nalgebra::Point2;
use serde::{Deserialize, Serialize};
use strum::{EnumIter, EnumProperty, IntoEnumIterator};

//
// Modes
//

#[derive(
    EnumIter, EnumProperty, strum_macros::Display, Default, Debug, PartialEq, Eq, Clone, Copy, Hash,
)]
#[repr(u8)]
pub enum EditorMode {
    #[default]
    #[strum(props(file_ext = "tunnel"))]
    Tunnels = 0,
    #[strum(props(file_ext = "room"))]
    Rooms = 1,
}

impl EditorMode {
    /// Determines what mode a file should use.
    /// Doesn't support uppercase file extensions because that's gross.
    pub fn from_path(path: &Path) -> anyhow::Result<Self> {
        let parts = path
            .to_str()
            .ok_or_else(|| anyhow!("invalid path"))?
            .split('.')
            .collect::<Vec<_>>();
        let len = parts.len();

        if len < 3 {
            return Err(anyhow!("filename doesn't have enough parts"));
        }

        let [mode, ext] = [parts[len - 2], parts[len - 1]];

        if ext != "ron" {
            return Err(anyhow!("not a ron file"));
        }

        for m in EditorMode::iter() {
            if m.get_str("file_ext") == Some(mode) {
                return Ok(m);
            }
        }

        Err(anyhow!("file extension not recognized"))
    }
}

#[derive(EnumIter, strum_macros::Display, Default, Debug, PartialEq, Eq, Clone, Copy, Hash)]
#[repr(u8)]
pub enum EditorViewMode {
    #[default]
    Editor = 0,
    Preview = 1,
}

//
// Tunnels mode
//

#[derive(Debug)]
pub struct TunnelsModeState {
    pub mirror: bool,
    pub selected_point: Option<usize>,
    pub drag_start: Option<(Point2<f32>, Vec2)>,
}

impl TunnelsModeState {
    pub fn dragging(&self) -> bool {
        self.drag_start.is_some()
    }
}

impl Default for TunnelsModeState {
    fn default() -> Self {
        Self {
            mirror: true,
            selected_point: None,
            drag_start: None,
        }
    }
}

//
// Rooms mode
//

#[derive(Debug)]
pub struct RoomsModeState {}

impl Default for RoomsModeState {
    fn default() -> Self {
        Self {}
    }
}

//
// File picker state
//

#[derive(Serialize, Deserialize, strum_macros::Display, Debug, Clone, PartialEq)]
pub enum FilePayload {
    Tunnel(Tunnel),
    Room(Room),
}

impl FilePayload {
    pub fn default_for_mode(mode: EditorMode) -> Self {
        match mode {
            EditorMode::Tunnels => Self::Tunnel(Tunnel::default()),
            EditorMode::Rooms => Self::Room(Room::default()),
        }
    }
}

#[derive(Debug)]
pub struct FilePickerState {
    pub directory: PathBuf,
    pub files: Vec<FileState>,
    pub filter: String,
    pub filter_mode: Option<EditorMode>,
    pub current: Option<usize>,
}

impl FilePickerState {
    pub fn file_ext_for_mode(mode: &EditorMode) -> String {
        format!(".{}.ron", mode.get_str("file_ext").unwrap())
    }

    pub fn file_name_for_mode(name: String, mode: &EditorMode) -> String {
        format!("{name}{}", Self::file_ext_for_mode(mode))
    }

    fn new_file_name(index: usize, mode: &EditorMode) -> String {
        format!("*{}{index}*", mode.get_str("file_ext").unwrap())
    }

    fn next_new_file_name(&self, mode: &EditorMode) -> String {
        let index = self.files.iter().filter(|file| file.path.is_none()).count();
        Self::new_file_name(index, mode)
    }

    pub fn current_file(&self) -> Option<&FileState> {
        self.current.map(|i| self.files.get(i))?
    }

    pub fn current_file_mut(&mut self) -> Option<&mut FileState> {
        self.current.map(|i| self.files.get_mut(i))?
    }

    pub fn current_data(&self) -> Option<&FilePayload> {
        self.current_file().map(|c| c.data.as_ref())?
    }

    pub fn current_data_mut(&mut self) -> Option<&mut FilePayload> {
        self.current_file_mut().map(|c| c.data.as_mut())?
    }

    pub fn switch_to_file(&mut self, index: usize) -> anyhow::Result<()> {
        if let Some(current_file) = self.current_file_mut() {
            if !current_file.changed && current_file.path.is_some() {
                current_file.data = None;
                current_file.last_saved_data = None;
            }
        }

        let file = self
            .files
            .get_mut(index)
            .ok_or_else(|| anyhow!("file does not exist"))?;

        self.current = Some(index);

        if file.data.is_some() {
            return Ok(());
        }

        if let Some(path) = file.path.clone() {
            file.read(path.clone())?;
        }

        Ok(())
    }

    pub fn revert_file(&mut self, index: usize) -> anyhow::Result<()> {
        let file = self
            .files
            .get_mut(index)
            .ok_or_else(|| anyhow!("file does not exist"))?;

        file.data = file.last_saved_data.clone();

        Ok(())
    }

    pub fn rename_file(&mut self, index: usize, name: String) -> anyhow::Result<()> {
        let file = self
            .files
            .get_mut(index)
            .ok_or_else(|| anyhow!("file does not exist"))?;

        let new_name = Self::file_name_for_mode(name, &file.mode);
        let new_path = self.directory.clone().join(PathBuf::from_str(&new_name)?);
        let old_name = file.name.clone();
        let old_path = file.path.clone().ok_or_else(|| anyhow!(""))?;

        let mut file = self.files.remove(index);
        file.path = Some(new_path.clone());
        file.name = new_name.clone();

        self.files
            .retain(|f| f.name != new_name && f.name != old_name);
        self.files.insert(0, file);
        self.current = Some(0);

        std::fs::rename(old_path, new_path)?;

        Ok(())
    }

    pub fn create_new_file(&mut self, mode: EditorMode) {
        self.files.insert(
            0,
            FileState {
                name: self.next_new_file_name(&mode),
                path: None,
                mode,
                changed: true,
                data: Some(FilePayload::default_for_mode(mode)),
                last_saved_data: Some(FilePayload::default_for_mode(mode)),
                modified_time: SystemTime::now(),
            },
        );
        self.current = Some(0);
    }

    /// Returns false if the file needs a path before it can be
    /// saved. UI should handle this by opening the "save as" dialog.
    pub fn save_file(&mut self, index: usize) -> anyhow::Result<bool> {
        let file = self
            .files
            .get_mut(index)
            .ok_or_else(|| anyhow!("file does not exist"))?;

        if file.path.is_none() {
            return Ok(false);
        }
        file.write()?;

        Ok(true)
    }

    pub fn save_file_with_name(&mut self, index: usize, name: String) -> anyhow::Result<()> {
        let current_file = self
            .files
            .get_mut(index)
            .ok_or_else(|| anyhow!("file does not exist"))?;

        let name = Self::file_name_for_mode(name, &current_file.mode);
        let old_path = current_file.path.clone();
        let path = PathBuf::from_str(&name).unwrap();
        let path = self.directory.clone().join(path);
        let is_new_file = current_file.path.is_none();

        let mut file = if is_new_file {
            self.files.remove(index)
        } else {
            if current_file.data.is_none() {
                current_file.read(old_path.unwrap())?;
            }
            current_file.clone()
        };

        file.path = Some(path);
        file.name = name.clone();
        file.write()?;
        self.files.retain(|f| f.name != name);
        self.files.insert(0, file);
        self.current = Some(0);

        Ok(())
    }

    pub fn save_current_file(&mut self) -> anyhow::Result<bool> {
        let Some(index) = self.current else {
            return Err(anyhow!("no current file index"));
        };

        self.save_file(index)
    }

    pub fn delete_file(&mut self, index: usize) -> anyhow::Result<()> {
        let file = self
            .files
            .get_mut(index)
            .ok_or_else(|| anyhow!("file does not exist"))?;

        if let Some(ref path) = file.path {
            std::fs::remove_file(path)?;
        }
        self.files.remove(index);

        if self.current == Some(index) {
            self.current = None;
        }

        Ok(())
    }

    pub fn from_directory(directory: &str) -> Self {
        // TODO move this elsewhere
        // TODO handle errors
        let directory = PathBuf::from_str(directory).unwrap();
        let files: Vec<FileState> = std::fs::read_dir(directory.clone())
            .unwrap()
            .filter_map(|f| {
                let f = f.unwrap();
                let name = f.file_name().into_string().unwrap();
                let modified_time = if let Ok(metadata) = f.metadata() {
                    metadata.modified().unwrap_or(SystemTime::now())
                } else {
                    SystemTime::now()
                };
                let mode = EditorMode::from_path(&f.path());

                if name.starts_with(".") || mode.is_err() {
                    None
                } else {
                    Some(FileState {
                        name,
                        mode: mode.unwrap(),
                        path: Some(f.path().clone()),
                        changed: false,
                        data: None,
                        last_saved_data: None,
                        modified_time,
                    })
                }
            })
            .collect();

        Self {
            files,
            directory,
            filter: String::new(),
            filter_mode: None,
            current: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileState {
    pub name: String,
    pub path: Option<PathBuf>,
    /// If data is None, it's because the file isn't loaded, not because it's empty.
    pub data: Option<FilePayload>,
    pub last_saved_data: Option<FilePayload>,
    pub mode: EditorMode,
    // Don't touch this, it's automatically updated by EditorModesPlugin.
    pub changed: bool,
    /// Only tracks the modified time according to the file metadata.
    pub modified_time: SystemTime,
}

impl FileState {
    pub fn read(&mut self, path: PathBuf) -> anyhow::Result<()> {
        if self.data.is_some() {
            return Err(anyhow!("tried to reread loaded file"));
        };

        let mut file = File::open(path.clone())?;
        let mut s = String::new();
        file.read_to_string(&mut s)?;

        self.data = Some(ron::from_str(&s)?);
        self.last_saved_data = self.data.clone();

        Ok(())
    }

    pub fn write(&mut self) -> anyhow::Result<()> {
        let Some(ref data) = self.data else {
            return Err(anyhow!("tried to write empty file"));
        };
        let Some(ref path) = self.path else {
            return Err(anyhow!("tried to save file with no path"));
        };

        let s = ron::ser::to_string_pretty(&data, ron::ser::PrettyConfig::default())?;
        let mut file = File::create(path.clone())?;
        file.write_all(s.as_bytes())?;

        self.modified_time = SystemTime::now();
        self.last_saved_data = self.data.clone();

        Ok(())
    }
}

//
// Miscellaneous
//

#[derive(Default, Debug, PartialEq)]
#[repr(u8)]
pub enum SpawnPickerMode {
    #[default]
    Inactive = 0,
    Picking = 1,
    Spawning = 2,
    Playing = 3,
    Despawning = 4,
}

#[derive(Default, Debug)]
pub struct SpawnPickerState {
    pub mode: SpawnPickerMode,
    pub position: Option<Vec3>,
}

//
// Main state
//

#[derive(Resource, Debug)]
pub struct EditorState {
    pub view: EditorViewMode,
    pub files: FilePickerState,
    pub spawn: SpawnPickerState,
    pub tunnels_mode: TunnelsModeState,
    #[allow(unused)]
    pub rooms_mode: RoomsModeState,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            view: Default::default(),
            files: FilePickerState::from_directory("assets/worldgen"),
            spawn: Default::default(),
            tunnels_mode: Default::default(),
            rooms_mode: Default::default(),
        }
    }
}

impl EditorState {
    // TODO maybe make this an Option?
    pub fn mode(&self) -> EditorMode {
        self.files
            .current_file()
            .map_or_else(|| EditorMode::Tunnels, |f| f.mode)
    }
}
