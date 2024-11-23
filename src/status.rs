#[derive(Clone, Debug, mlua::FromLua)]
pub enum Duration {
	Rest,
	Turn,
}

impl mlua::UserData for Duration {}

#[derive(Clone, Debug)]
pub struct Status {
	pub name: String,
	pub icon: String,
	pub duration: Duration,
	pub on_debuff: Option<mlua::Function>,
}

impl mlua::UserData for Status {}
