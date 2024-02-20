use crate::character;
use sdl2::image::LoadTexture;
use sdl2::render::{Texture, TextureCreator};
use sdl2::video::WindowContext;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::{fs, io};

#[derive(Debug, thiserror::Error)]
pub enum ResourceManagerError {
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("{0}")]
    Toml(#[from] toml::de::Error),
    #[error("{0}")]
    Texture(String),
}

pub struct ResourceManager<'texture> {
    sheets: HashMap<PathBuf, character::Sheet>,
    textures: HashMap<PathBuf, Texture<'texture>>,
}

impl<'texture> ResourceManager<'texture> {
    pub fn open(
        path: impl AsRef<Path>,
        texture_creator: &'texture TextureCreator<WindowContext>,
    ) -> Result<ResourceManager<'texture>, ResourceManagerError> {
        let path = path.as_ref();
        macro_rules! open_toml_directory {
            ($name:ident, $dir:expr) => {
                let directory = path.join($dir);
                for entry in fs::read_dir(&directory)? {
                    let path = entry?.path();
                    let resource = toml::from_str(&fs::read_to_string(&path)?)?;
                    let reference = path
                        .strip_prefix(&directory)
                        .map(PathBuf::from)
                        .unwrap_or_default()
                        .parent()
                        .unwrap_or(&Path::new(""))
                        .join(path.file_prefix().unwrap());
                    $name.insert(reference, resource);
                }
            };
        }
        let mut sheets = HashMap::new();
        open_toml_directory!(sheets, "sheets/");
        let mut textures = HashMap::new();
        let texture_directory = Path::new("textures/");
        let directory = path.join(texture_directory);
        for entry in fs::read_dir(&directory)? {
            let path = entry?.path();
            let texture = texture_creator
                .load_texture(&path)
                .map_err(ResourceManagerError::Texture)?;
            let reference = path
                .strip_prefix(&directory)
                .map(PathBuf::from)
                .unwrap_or_default()
                .parent()
                .unwrap_or(Path::new(""))
                .join(path.file_prefix().unwrap());
            textures.insert(reference, texture);
        }
        Ok(Self { sheets, textures })
    }

    pub fn get_sheet(&self, path: impl AsRef<Path>) -> Option<&character::Sheet> {
        self.sheets.get(path.as_ref())
    }

    pub fn get_texture(&self, path: impl AsRef<Path>) -> Option<&Texture> {
        self.textures.get(path.as_ref())
    }
}
