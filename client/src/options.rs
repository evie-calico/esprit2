use esprit2::prelude::*;
use sdl3::keyboard::Keycode;
use std::path::Path;
use std::sync::OnceLock;
use std::{fs, io};

pub(crate) fn user_directory() -> &'static Path {
	static USER_DIRECTORY: OnceLock<&'static Path> = OnceLock::new();
	USER_DIRECTORY.get_or_init(find_user_directory)
}

pub(crate) fn resource_directory() -> &'static Path {
	static RESOURCE_DIRECTORY: OnceLock<&'static Path> = OnceLock::new();
	RESOURCE_DIRECTORY.get_or_init(find_resource_directory)
}

// In the future, this should be a little smarter.
// Things to check:
// - ~/.local/share/esprit2 (XDG_DATA_HOME)
fn find_user_directory() -> &'static Path {
	Path::new("user/")
}

// I think `local/share` is still the answer here,
// but we need to check /usr/local/share/esprit2 if this program is installed system-wide.
// This isn't the case for `find_user_directory` since /usr/local/share might not be writable.
fn find_resource_directory() -> &'static Path {
	Path::new("res/")
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct Options {
	pub(crate) board: Board,
	pub(crate) ui: UserInterface,
	pub(crate) controls: Controls,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum OpenOptionsError {
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
	pub(crate) fn open(path: impl AsRef<Path>) -> Result<Self, OpenOptionsError> {
		Ok(toml::from_str(&fs::read_to_string(path)?)?)
	}
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct Board {
	pub(crate) scale: u32,
}

impl Default for Board {
	fn default() -> Self {
		Self { scale: 3 }
	}
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct UserInterface {
	pub(crate) colors: Colors,

	pub(crate) pamphlet_width: u32,
	pub(crate) console_height: u32,
}

impl Default for UserInterface {
	fn default() -> Self {
		Self {
			colors: Colors::default(),

			pamphlet_width: 400,
			console_height: 200,
		}
	}
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct ConsoleColors {
	pub(crate) normal: Color,
	pub(crate) system: Color,
	pub(crate) unimportant: Color,
	pub(crate) defeat: Color,
	pub(crate) danger: Color,
	pub(crate) important: Color,
	pub(crate) special: Color,
	pub(crate) combat: Color,
}

impl Default for ConsoleColors {
	fn default() -> Self {
		Self {
			normal: (255, 255, 255, 255),
			system: (100, 100, 100, 255),
			unimportant: (100, 100, 100, 255),
			defeat: (255, 128, 128, 255),
			danger: (255, 0, 0, 255),
			important: (255, 255, 0, 255),
			special: (0, 255, 0, 255),
			combat: (255, 255, 128, 255),
		}
	}
}

/// User interfact colors
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct Colors {
	pub(crate) normal_mode: Color,
	pub(crate) select_mode: Color,
	pub(crate) prompt_mode: Color,
	pub(crate) console: ConsoleColors,
}

impl Default for Colors {
	fn default() -> Self {
		Self {
			normal_mode: (0x77, 0xE7, 0xA2, 0xFF),
			select_mode: (0xA2, 0x77, 0xE7, 0xFF),
			prompt_mode: (0xE7, 0xA2, 0x77, 0xFF),
			console: ConsoleColors::default(),
		}
	}
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Key(Keycode);

impl serde::Serialize for Key {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		serializer.serialize_str(&self.0.name())
	}
}

struct KeyVisitor;

impl serde::de::Visitor<'_> for KeyVisitor {
	type Value = String;

	fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
		formatter.write_str("SDL3 keycode name")
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
pub(crate) struct Triggers(Vec<Key>);

impl Triggers {
	pub(crate) fn contains(&self, keycode: Keycode) -> bool {
		self.0.iter().any(|x| x.0 == keycode)
	}
}

impl std::fmt::Display for Triggers {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let mut triggers = self.0.iter();
		if let Some(first) = triggers.next() {
			write!(f, "{}", first.0.name())?;
			for i in triggers {
				write!(f, ", {}", i.0.name())?;
			}
		}
		Ok(())
	}
}

impl std::ops::Deref for Triggers {
	type Target = Vec<Key>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct Controls {
	pub(crate) left: Triggers,
	pub(crate) right: Triggers,
	pub(crate) up: Triggers,
	pub(crate) down: Triggers,
	pub(crate) up_left: Triggers,
	pub(crate) up_right: Triggers,
	pub(crate) down_left: Triggers,
	pub(crate) down_right: Triggers,

	pub(crate) talk: Triggers,
	pub(crate) autocombat: Triggers,
	pub(crate) select: Triggers,
	pub(crate) attack: Triggers,
	pub(crate) act: Triggers,
	pub(crate) underfoot: Triggers,

	pub(crate) yes: Triggers,
	pub(crate) no: Triggers,
	pub(crate) confirm: Triggers,
	pub(crate) escape: Triggers,
	pub(crate) fullscreen: Triggers,
	pub(crate) debug: Triggers,
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
			autocombat: Triggers(vec![Key(K::Tab)]),
			select: Triggers(vec![Key(K::F)]),
			attack: Triggers(vec![Key(K::V)]),
			act: Triggers(vec![Key(K::C)]),
			underfoot: Triggers(vec![Key(K::Period)]),

			yes: Triggers(vec![Key(K::Y)]),
			no: Triggers(vec![Key(K::N)]),
			confirm: Triggers(vec![Key(K::Return)]),
			escape: Triggers(vec![Key(K::Escape)]),
			fullscreen: Triggers(vec![Key(K::F11)]),
			debug: Triggers(vec![Key(K::F1)]),
		}
	}
}
