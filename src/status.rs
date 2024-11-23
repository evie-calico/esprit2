#[derive(
	Clone,
	Debug,
	serde::Serialize,
	serde::Deserialize,
	rkyv::Archive,
	rkyv::Serialize,
	rkyv::Deserialize,
)]
pub enum Duration {
	Rest,
	Turn,
}

#[derive(
	Clone,
	Debug,
	serde::Serialize,
	serde::Deserialize,
	rkyv::Archive,
	rkyv::Serialize,
	rkyv::Deserialize,
)]
pub struct Status {
	pub name: String,
	pub icon: String,
	pub duration: Duration,
	pub on_debuff: Box<str>,
}

impl mlua::UserData for Status {}
