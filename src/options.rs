use crate::prelude::*;
use sdl2::keyboard::Keycode;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::{fs, io};

pub fn user_directory() -> &'static PathBuf {
	static USER_DIRECTORY: OnceLock<PathBuf> = OnceLock::new();
	USER_DIRECTORY.get_or_init(find_user_directory)
}

pub fn resource_directory() -> &'static PathBuf {
	static RESOURCE_DIRECTORY: OnceLock<PathBuf> = OnceLock::new();
	RESOURCE_DIRECTORY.get_or_init(find_resource_directory)
}

// In the future, this should be a little smarter.
// Things to check:
// - ~/.local/share/esprit2 (XDG_DATA_HOME)
fn find_user_directory() -> PathBuf {
	PathBuf::from("user/")
}

// I think `local/share` is still the answer here,
// but we need to check /usr/local/share/esprit2 if this program is installed system-wide.
// This isn't the case for `find_user_directory` since /usr/local/share might not be writable.
fn find_resource_directory() -> PathBuf {
	PathBuf::from("res/")
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Options {
	pub ui: UserInterface,
	pub controls: Controls,
}

#[derive(Debug, thiserror::Error)]
pub enum OpenOptionsError {
	#[error("{0}")]
	Io(#[from] io::Error),
	#[error("{0}")]
	Toml(#[from] toml::de::Error),
}

impl Options {
	/// Open and return an options file.
	///
	/// # Errors
	///
	/// Fails if the file could not be opened or parsed.
	pub fn open(path: impl AsRef<Path>) -> Result<Self, OpenOptionsError> {
		Ok(toml::from_str(&fs::read_to_string(path)?)?)
	}
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct UserInterface {
	pub colors: Colors,
	pub typography: typography::Options,

	pub pamphlet_width: u32,
	pub console_height: u32,
}

impl Default for UserInterface {
	fn default() -> Self {
		Self {
			colors: Colors::default(),
			typography: typography::Options::default(),

			pamphlet_width: 400,
			console_height: 200,
		}
	}
}

/// User interfact colors
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Colors {
	pub normal_mode: Color,
	pub cast_mode: Color,
	pub cursor_mode: Color,
	pub console: console::Colors,
}

impl Default for Colors {
	fn default() -> Self {
		Self {
			normal_mode: (0x77, 0xE7, 0xA2, 0xFF),
			cast_mode: (0xA2, 0x77, 0xE7, 0xFF),
			cursor_mode: (0xE7, 0xA2, 0x77, 0xFF),
			console: console::Colors::default(),
		}
	}
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Key(Keycode);

impl serde::Serialize for Key {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		serializer.serialize_str(&self.0.name())
	}
}

struct KeyVisitor;

impl<'de> serde::de::Visitor<'de> for KeyVisitor {
	type Value = String;

	fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
		formatter.write_str("a string containing an expression")
	}

	fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
	where
		E: serde::de::Error,
	{
		Ok(value)
	}

	fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
	where
		E: serde::de::Error,
	{
		Ok(value.to_string())
	}
}

impl<'de> serde::Deserialize<'de> for Key {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		use serde::de::Error;
		Ok(Key(Keycode::from_name(
			&deserializer.deserialize_string(KeyVisitor)?,
		)
		.ok_or(D::Error::custom("unknown key name"))?))
	}
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Triggers(Vec<Key>);

impl Triggers {
	pub fn contains(&self, keycode: Keycode) -> bool {
		self.0.iter().any(|x| x.0 == keycode)
	}
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Controls {
	pub left: Triggers,
	pub right: Triggers,
	pub up: Triggers,
	pub down: Triggers,
	pub up_left: Triggers,
	pub up_right: Triggers,
	pub down_left: Triggers,
	pub down_right: Triggers,

	pub talk: Triggers,
	pub attack: Triggers,
	pub cast: Triggers,
	pub underfoot: Triggers,

	pub confirm: Triggers,
	pub escape: Triggers,
	pub fullscreen: Triggers,
	pub debug: Triggers,
}

impl Default for Controls {
	fn default() -> Self {
		use Keycode as K;

		Self {
			left: Triggers(vec![Key(K::H), Key(K::Left), Key(K::Kp4)]),
			right: Triggers(vec![Key(K::L), Key(K::Right), Key(K::Kp6)]),
			up: Triggers(vec![Key(K::K), Key(K::Up), Key(K::Kp8)]),
			down: Triggers(vec![Key(K::J), Key(K::Down), Key(K::Kp2)]),
			up_left: Triggers(vec![Key(K::Y), Key(K::Kp7)]),
			up_right: Triggers(vec![Key(K::U), Key(K::Kp9)]),
			down_left: Triggers(vec![Key(K::B), Key(K::Kp1)]),
			down_right: Triggers(vec![Key(K::N), Key(K::Kp3)]),

			talk: Triggers(vec![Key(K::T)]),
			attack: Triggers(vec![Key(K::V)]),
			cast: Triggers(vec![Key(K::C)]),
			underfoot: Triggers(vec![Key(K::Period)]),

			confirm: Triggers(vec![Key(K::Return)]),
			escape: Triggers(vec![Key(K::Escape)]),
			fullscreen: Triggers(vec![Key(K::F11)]),
			debug: Triggers(vec![Key(K::F1)]),
		}
	}
}

/// Potentially useful information for assinging lettered shortcuts for a list.
///
/// Does not (currently) support shifted letters; they're probably necessary but I don't know how I feel about it yet.
pub struct Shortcut {
	pub symbol: char,
	pub keycode: Keycode,
}

impl TryFrom<usize> for Shortcut {
	type Error = ();

	fn try_from(index: usize) -> Result<Self, ()> {
		// i32 is the most restrictive value we use (actually, a u5 would be fine—we only care about 0-25)
		// However, it makes sense for this function to accept a usize considering this is for lettering indices.
		let Ok::<i32, _>(index) = index.try_into() else {
			return Err(());
		};
		let Some(symbol) = char::from_digit(10 + (index as u32), 36) else {
			return Err(());
		};
		// This unwrap is safe because the above succeeded.
		let keycode = Keycode::from_i32(Keycode::A.into_i32() + index)
			.expect("symbol must be within the valid keycode range");
		Ok(Self { symbol, keycode })
	}
}
