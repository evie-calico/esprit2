use crate::{attack::Attack, character};
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
    missing_texture: Texture<'texture>,
    attacks: HashMap<PathBuf, Attack>,
    sheets: HashMap<PathBuf, character::Sheet>,
    textures: HashMap<PathBuf, Texture<'texture>>,
}

fn begin_recurse<T>(
    container: &mut HashMap<PathBuf, T>,
    directory: &Path,
    loader: &dyn Fn(&Path) -> Result<T, ResourceManagerError>,
) -> Result<(), ResourceManagerError> {
    recurse(container, directory, directory, loader)
}
fn recurse<T>(
    container: &mut HashMap<PathBuf, T>,
    base_directory: &Path,
    directory: &Path,
    loader: &dyn Fn(&Path) -> Result<T, ResourceManagerError>,
) -> Result<(), ResourceManagerError> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        if entry.metadata()?.is_dir() {
            recurse(container, base_directory, &path, loader)?;
        } else {
            let resource = loader(&path)?;
            let reference = path
                .strip_prefix(base_directory)
                .map(PathBuf::from)
                .unwrap_or_default()
                .parent()
                .unwrap_or(Path::new(""))
                .join(path.file_prefix().unwrap());
            container.insert(reference, resource);
        }
    }
    Ok(())
}

impl<'texture> ResourceManager<'texture> {
    pub fn open(
        path: impl AsRef<Path>,
        texture_creator: &'texture TextureCreator<WindowContext>,
    ) -> Result<ResourceManager<'texture>, ResourceManagerError> {
        let path = path.as_ref();

        let mut sheets = HashMap::new();
        begin_recurse(&mut sheets, &path.join("sheets"), &|path| {
            Ok(toml::from_str(&fs::read_to_string(path)?)?)
        })?;

        let mut attacks = HashMap::new();
        begin_recurse(&mut attacks, &path.join("attacks"), &|path| {
            Ok(toml::from_str(&fs::read_to_string(path)?)?)
        })?;

        let mut textures = HashMap::new();
        begin_recurse(&mut textures, &path.join("textures"), &|path| {
            texture_creator
                .load_texture(path)
                .map_err(ResourceManagerError::Texture)
        })?;

        // Include a missing texture placeholder, rather than returning an Option.
        let missing_texture = texture_creator
            .load_texture_bytes(include_bytes!("res/missing_texture.png"))
            .unwrap();

        Ok(Self {
            attacks,
            sheets,
            textures,
            missing_texture,
        })
    }

    pub fn get_sheet(&self, path: impl AsRef<Path>) -> Option<&character::Sheet> {
        self.sheets.get(path.as_ref())
    }

    pub fn get_attack(&self, path: impl AsRef<Path>) -> Option<&Attack> {
        self.attacks.get(path.as_ref())
    }

    pub fn get_texture(&self, path: impl AsRef<Path>) -> &Texture {
        self.textures
            .get(path.as_ref())
            .unwrap_or(&self.missing_texture)
    }
}
