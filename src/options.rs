use sdl2::keyboard::Keycode;
use std::path::PathBuf;

/// SDL2 Keycodes do not implement serde traits,
/// but they can be converted to and from i32s.
/// Thus, this type should be used in their place.
pub type KeycodeIndex = i32;

lazy_static::lazy_static! {
    pub static ref USER_DIRECTORY: PathBuf = get_user_directory();
    pub static ref RESOURCE_DIRECTORY: PathBuf = get_resource_directory();
}

// In the future, this should be a little smarter.
// Honestly I'm not sure if lazy_static is even the right choice because it precludes the use of clap.
// I guess I could creeate another clap parser that ignores everything except --user?
fn get_user_directory() -> PathBuf {
    PathBuf::from("user/")
}

fn get_resource_directory() -> PathBuf {
    PathBuf::from("res/")
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Options {
    pub controls: Controls,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Controls {
    pub left: Vec<KeycodeIndex>,
    pub right: Vec<KeycodeIndex>,
    pub up: Vec<KeycodeIndex>,
    pub down: Vec<KeycodeIndex>,
    pub up_left: Vec<KeycodeIndex>,
    pub up_right: Vec<KeycodeIndex>,
    pub down_left: Vec<KeycodeIndex>,
    pub down_right: Vec<KeycodeIndex>,
}

impl Default for Controls {
    fn default() -> Self {
        use Keycode as K;
        Self {
            left: vec![K::H as i32, K::Left as i32, K::Kp4 as i32],
            right: vec![K::L as i32, K::Right as i32, K::Kp6 as i32],
            up: vec![K::K as i32, K::Up as i32, K::Kp8 as i32],
            down: vec![K::J as i32, K::Down as i32, K::Kp2 as i32],
            up_left: vec![K::Y as i32, K::Kp7 as i32],
            up_right: vec![K::U as i32, K::Kp9 as i32],
            down_left: vec![K::B as i32, K::Kp1 as i32],
            down_right: vec![K::N as i32, K::Kp3 as i32],
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
        // clever, huh?
        let Some(symbol) = char::from_digit(10 + (index as u32), 36) else {
            return Err(());
        };
        // This unwrap is safe because the above succeeded.
        let keycode = Keycode::from_i32(Keycode::A as i32 + index).unwrap();
        Ok(Self { symbol, keycode })
    }
}
