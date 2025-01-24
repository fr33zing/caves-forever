use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
    path::PathBuf,
    str::FromStr,
    time::SystemTime,
};

use anyhow::anyhow;
use bevy::prelude::*;
use mines::worldgen::asset::Tunnel;
use nalgebra::Point2;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use strum::EnumIter;

//
// Modes
//

#[derive(EnumIter, strum_macros::Display, Default, Debug, PartialEq, Eq, Clone, Copy, Hash)]
#[repr(u8)]
pub enum EditorMode {
    #[default]
    Tunnels = 0,
    Rooms = 1,
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
    pub files: FilePickerState<Tunnel>,
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
            files: FilePickerState::from_directory("assets/worldgen/tunnels"),
            mirror: true,
            selected_point: None,
            drag_start: None,
        }
    }
}

//
// Rooms mode
//

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Room {}

impl Default for Room {
    fn default() -> Self {
        Self {}
    }
}

#[derive(Debug)]
pub struct RoomsModeState {
    pub files: FilePickerState<Room>,
}

impl Default for RoomsModeState {
    fn default() -> Self {
        Self {
            files: FilePickerState::from_directory("assets/worldgen/rooms"),
        }
    }
}

//
// File picker state
//

#[derive(Debug)]
pub struct FilePickerState<T>
where
    T: Serialize + DeserializeOwned + Default + Clone,
{
    pub directory: PathBuf,
    pub files: Vec<FileState<T>>,
    pub filter: String,
    pub current: usize,
}

impl<T> FilePickerState<T>
where
    T: Serialize + DeserializeOwned + Default + Clone,
{
    pub fn current_file(&self) -> &FileState<T> {
        self.files
            .get(self.current)
            .expect("current file does not exist")
    }

    pub fn current_file_mut(&mut self) -> &mut FileState<T> {
        self.files
            .get_mut(self.current)
            .expect("current file does not exist")
    }

    pub fn current_data(&self) -> &Option<T> {
        &self.current_file().data
    }

    pub fn current_data_mut(&mut self) -> Option<&mut T> {
        let Some(ref mut data) = self.current_file_mut().data else {
            return None;
        };
        Some(data)
    }

    pub fn switch_to_file(&mut self, index: usize) -> anyhow::Result<()> {
        let current_file = self.current_file_mut();
        if !current_file.changed {
            current_file.data = None;
        }

        let file = self
            .files
            .get_mut(index)
            .ok_or_else(|| anyhow!("file does not exist"))?;

        if file.data.is_some() {
            self.current = index;
            return Ok(());
        }

        if let Some(path) = file.path.clone() {
            file.read(path.clone())?;
            self.current = index;
        } else {
            self.files.insert(0, Default::default());
            self.current = 0;
        }

        Ok(())
    }
}

impl<T> FilePickerState<T>
where
    T: Serialize + DeserializeOwned + Default + Clone,
{
    fn new_file_name(index: usize) -> String {
        format!("* untitled{index} *")
    }

    fn next_new_file_name(&self) -> String {
        let index = self.files.iter().filter(|file| file.path.is_none()).count();
        Self::new_file_name(index)
    }

    pub fn create_new_file(&mut self) {
        self.files.insert(
            0,
            FileState {
                name: self.next_new_file_name(),
                path: None,
                changed: true,
                data: Some(T::default()),
                modified_time: SystemTime::now(),
            },
        );
        self.current = 0;
    }

    /// Returns false if the current file needs a path before it can
    /// be saved. UI should handle this by opening the "save as" dialog.
    pub fn save_current_file(&mut self) -> anyhow::Result<bool> {
        let file = self.current_file_mut();
        if file.path.is_none() {
            return Ok(false);
        }
        file.write()?;

        Ok(true)
    }

    pub fn save_current_file_with_name(&mut self, name: String) -> anyhow::Result<()> {
        let path = PathBuf::from_str(&name).unwrap();
        let path = self.directory.clone().join(path);
        let is_new_file = self.current_file().path.is_none();

        let mut file = if is_new_file {
            let mut file = self.files.remove(self.current);
            file.changed = false;

            file
        } else {
            let old_file = self.current_file_mut();
            old_file.changed = false;
            let new_file = old_file.clone();
            old_file.data = None;

            new_file
        };

        file.path = Some(path);
        file.name = name.clone();
        file.write()?;
        self.files.insert(0, file);
        self.current = 0;

        Ok(())
    }

    pub fn from_directory(directory: &str) -> Self {
        // TODO move this elsewhere
        let directory = PathBuf::from_str(directory).unwrap();
        let mut files: Vec<FileState<T>> = std::fs::read_dir(directory.clone())
            .unwrap()
            .filter_map(|f| {
                let f = f.unwrap();
                let name = f.file_name().into_string().unwrap();
                let modified_time = if let Ok(metadata) = f.metadata() {
                    metadata.modified().unwrap_or(SystemTime::now())
                } else {
                    SystemTime::now()
                };

                if name.starts_with(".") {
                    None
                } else {
                    Some(FileState {
                        name,
                        path: Some(f.path().clone()),
                        changed: false,
                        data: None,
                        modified_time,
                    })
                }
            })
            .collect();

        files.insert(
            0,
            FileState {
                name: Self::new_file_name(0),
                path: None,
                changed: true,
                data: Some(T::default()),
                modified_time: SystemTime::now(),
            },
        );

        Self {
            files,
            directory,
            filter: String::new(),
            current: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileState<T>
where
    T: Serialize + DeserializeOwned + Default + Clone,
{
    pub name: String,
    pub path: Option<PathBuf>,
    pub data: Option<T>,
    pub changed: bool,
    /// Only tracks the modified time according to the file metadata
    pub modified_time: SystemTime,
}

impl<T> Default for FileState<T>
where
    T: Serialize + DeserializeOwned + Default + Clone,
{
    fn default() -> Self {
        Self {
            name: Default::default(),
            path: Default::default(),
            data: Default::default(),
            changed: Default::default(),
            modified_time: SystemTime::now(),
        }
    }
}

impl<T> FileState<T>
where
    T: Serialize + DeserializeOwned + Default + Clone,
{
    pub fn read(&mut self, path: PathBuf) -> anyhow::Result<()> {
        if self.data.is_some() {
            return Err(anyhow!("tried to reread loaded file"));
        };

        let mut file = File::open(path.clone())?;
        let mut s = String::new();
        file.read_to_string(&mut s)?;

        self.data = Some(ron::from_str(&s)?);
        self.changed = false;

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
        self.changed = false;

        Ok(())
    }
}

//
// Main state
//

#[derive(Resource, Default, Debug)]
pub struct EditorState {
    pub mode: EditorMode,
    pub view: EditorViewMode,
    pub tunnels_mode: TunnelsModeState,
    pub rooms_mode: RoomsModeState,
}

impl EditorState {
    /// Determine if the user has selected an object
    pub fn has_selection(&self) -> bool {
        match self.mode {
            EditorMode::Tunnels => {
                self.tunnels_mode.selected_point.is_some() && !self.tunnels_mode.dragging()
            }
            _ => false,
        }
    }

    pub fn file_picker(
        &self,
    ) -> Option<&FilePickerState<impl Serialize + DeserializeOwned + Default + Clone>> {
        match self.mode {
            EditorMode::Tunnels => Some(&self.tunnels_mode.files),
            _ => None,
        }
    }

    pub fn file_picker_mut(
        &mut self,
    ) -> Option<&mut FilePickerState<impl Serialize + DeserializeOwned + Default + Clone>> {
        match self.mode {
            EditorMode::Tunnels => Some(&mut self.tunnels_mode.files),
            _ => None,
        }
    }
}
