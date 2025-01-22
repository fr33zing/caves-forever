use std::{
    collections::HashMap,
    f32::consts::PI,
    fs::File,
    io::{Read, Write},
    path::PathBuf,
    str::FromStr,
    time::SystemTime,
};

use anyhow::anyhow;
use bevy::prelude::*;
use nalgebra::Point2;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use strum::{EnumIter, EnumProperty};

//
// Modes
//

#[derive(EnumProperty, EnumIter, Default, Debug, PartialEq, Eq, Clone, Copy, Hash)]
#[repr(u8)]
pub enum EditorMode {
    #[default]
    #[strum(props(Name = "Tunnels"))]
    Tunnels = 0,

    #[strum(props(Name = "Rooms"))]
    Rooms = 1,
}

#[derive(EnumProperty, EnumIter, Default, Debug, PartialEq, Eq, Clone, Copy, Hash)]
#[repr(u8)]
pub enum EditorViewMode {
    #[default]
    #[strum(props(Name = "Editor"))]
    Editor = 0,

    #[strum(props(Name = "Preview"))]
    Preview = 1,
}

//
// Tunnels mode
//

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TunnelProfile {
    pub points: Vec<Point2<f32>>,
}

impl Default for TunnelProfile {
    fn default() -> Self {
        let n = 10;
        let radius = 5.0;
        let mut points = Vec::<Point2<f32>>::new();

        for i in 0..n {
            let radians = (i as f32 / n as f32) * PI * 2.0;
            points.push(Point2::new(radians.sin(), -radians.cos()) * radius);
        }

        Self { points }
    }
}

#[derive(Debug)]
pub struct TunnelsModeState {
    pub files: FilePickerState<TunnelProfile>,
    pub mirror: bool,
    pub drag_point: Option<usize>,
    pub drag_start: Option<(Point2<f32>, Vec2)>,
}

impl Default for TunnelsModeState {
    fn default() -> Self {
        Self {
            files: FilePickerState::from_directory("assets/worldgen/tunnels"),
            mirror: true,
            drag_point: None,
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
    pub files: HashMap<Option<PathBuf>, FileState<T>>,
    pub filter: String,
    pub current: Option<PathBuf>,
}

impl<T> FilePickerState<T>
where
    T: Serialize + DeserializeOwned + Default + Clone,
{
    pub fn current_file(&self) -> Option<&FileState<T>> {
        self.files.get(&self.current)
    }

    pub fn current_file_mut(&mut self) -> Option<&mut FileState<T>> {
        self.files.get_mut(&self.current)
    }

    pub fn current_data(&self) -> &Option<T> {
        let Some(file) = self.current_file() else {
            return &None;
        };
        &file.data
    }

    pub fn current_data_mut(&mut self) -> Option<&mut T> {
        let Some(file) = self.current_file_mut() else {
            return None;
        };
        let Some(ref mut data) = file.data else {
            return None;
        };
        Some(data)
    }
}

impl<T> FilePickerState<T>
where
    T: Serialize + DeserializeOwned + Default + Clone,
{
    pub fn from_directory(directory: &str) -> Self {
        // TODO move this elsewhere
        let directory = PathBuf::from_str(directory).unwrap();
        let mut files: HashMap<Option<PathBuf>, FileState<T>> =
            std::fs::read_dir(directory.clone())
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
                        Some((
                            Some(f.path().clone()),
                            FileState {
                                name,
                                changed: false,
                                data: None,
                                modified_time,
                            },
                        ))
                    }
                })
                .collect();

        files.insert(
            None,
            FileState {
                name: "*untitled*".into(),
                changed: true,
                data: Some(T::default()),
                modified_time: SystemTime::now(),
            },
        );

        Self {
            files,
            directory,
            filter: String::new(),
            current: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileState<T>
where
    T: Serialize + DeserializeOwned + Default + Clone,
{
    pub name: String,
    pub changed: bool,
    pub data: Option<T>,
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
            changed: Default::default(),
            data: Default::default(),
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

        self.data = ron::from_str(&s)?;
        self.changed = false;

        Ok(())
    }

    pub fn write(&mut self, path: PathBuf) -> anyhow::Result<()> {
        let Some(ref data) = self.data else {
            return Err(anyhow!("tried to write empty file"));
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
