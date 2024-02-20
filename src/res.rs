use crate::{character, options::RESOURCE_DIRECTORY};
use sdl2::image::LoadTexture;
use sdl2::render::{Texture, TextureCreator};
use sdl2::video::WindowContext;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::{fs, io};

pub struct ResourceManager<'texture> {
    texture_creator: &'texture TextureCreator<WindowContext>,
    sheets: HashMap<PathBuf, character::Sheet>,
    textures: HashMap<PathBuf, Texture<'texture>>,
}

impl<'texture> ResourceManager<'texture> {
    pub fn open(
        path: impl AsRef<Path>,
        texture_creator: &'texture TextureCreator<WindowContext>,
    ) -> io::Result<ResourceManager<'texture>> {
        macro_rules! open_toml_directory {
            ($name:ident, $dir:expr) => {
                let directory = RESOURCE_DIRECTORY.join($dir);
                for entry in fs::read_dir(&directory)? {
                    let path = entry.unwrap().path();
                    let resource = toml::from_str(&fs::read_to_string(&path)?).unwrap();
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
        let directory = RESOURCE_DIRECTORY.join(texture_directory);
        for entry in fs::read_dir(&directory)? {
            let path = entry.unwrap().path();
            let texture = texture_creator.load_texture(&path).unwrap();
            let reference = path
                .strip_prefix(&directory)
                .map(PathBuf::from)
                .unwrap_or_default()
                .parent()
                .unwrap_or(&Path::new(""))
                .join(path.file_prefix().unwrap());
            textures.insert(reference, texture);
        }
        Ok(Self {
            texture_creator,
            sheets,
            textures,
        })
    }

    pub fn get_sheet(&self, path: impl AsRef<Path>) -> Option<&character::Sheet> {
        self.sheets.get(path.as_ref())
    }

    pub fn get_texture(&self, path: impl AsRef<Path>) -> Option<&Texture> {
        self.textures.get(path.as_ref())
    }
}
