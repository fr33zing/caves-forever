use std::{
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
    str::FromStr,
    time::SystemTime,
};

use anyhow::anyhow;
use bevy::prelude::*;
use mines::worldgen::asset::Tunnel;
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Room {}

impl Default for Room {
    fn default() -> Self {
        Self {}
    }
}

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

    pub fn from_path(path: PathBuf) -> anyhow::Result<Self> {
        let payload = match EditorMode::from_path(&path)? {
            EditorMode::Tunnels => Self::Tunnel(Tunnel::default()),
            EditorMode::Rooms => Self::Room(Room::default()),
        };
        Ok(payload)
    }
}

#[derive(Debug)]
pub struct FilePickerState {
    pub directory: PathBuf,
    pub files: Vec<FileState>,
    pub filter: String,
    pub current: Option<usize>,
}

impl FilePickerState {
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
}

impl FilePickerState {
    fn new_file_name(index: usize, mode: &EditorMode) -> String {
        format!("*{}{index}*", mode.get_str("file_ext").unwrap())
    }

    fn next_new_file_name(&self, mode: &EditorMode) -> String {
        let index = self.files.iter().filter(|file| file.path.is_none()).count();
        Self::new_file_name(index, mode)
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

    /// Returns false if the current file needs a path before it can
    /// be saved. UI should handle this by opening the "save as" dialog.
    pub fn save_current_file(&mut self) -> anyhow::Result<bool> {
        let Some(current_file) = self.current_file_mut() else {
            return Err(anyhow!("no current file"));
        };
        if current_file.path.is_none() {
            return Ok(false);
        }
        current_file.write()?;

        Ok(true)
    }

    pub fn save_current_file_with_name(&mut self, name: String) -> anyhow::Result<()> {
        let Some(current_file_index) = self.current else {
            return Err(anyhow!("no current file index"));
        };
        let Some(current_file) = self.current_file() else {
            return Err(anyhow!("no current file"));
        };

        let name = format!(
            "{name}.{}.ron",
            current_file.mode.get_str("file_ext").unwrap()
        );
        let path = PathBuf::from_str(&name).unwrap();
        let path = self.directory.clone().join(path);
        let is_new_file = current_file.path.is_none();

        let mut file = if is_new_file {
            self.files.remove(current_file_index)
        } else {
            let old_file = self.current_file_mut().unwrap();
            let new_file = old_file.clone();
            old_file.data = None;

            new_file
        };

        file.path = Some(path);
        file.name = name.clone();
        file.write()?;
        self.files.insert(0, file);
        self.current = Some(0);

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

        let s = ron::to_string(&data)?;
        let mut file = File::create(path.clone())?;
        file.write_all(s.as_bytes())?;

        self.modified_time = SystemTime::now();
        self.last_saved_data = self.data.clone();

        Ok(())
    }
}

//
// Main state
//

#[derive(Resource, Debug)]
pub struct EditorState {
    pub view: EditorViewMode,
    pub files: FilePickerState,
    pub tunnels_mode: TunnelsModeState,
    pub rooms_mode: RoomsModeState,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            view: Default::default(),
            tunnels_mode: Default::default(),
            rooms_mode: Default::default(),
            files: FilePickerState::from_directory("assets/worldgen"),
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
