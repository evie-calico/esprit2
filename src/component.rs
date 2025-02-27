#[derive(Clone, Debug)]
pub struct Component {
	pub name: String,
	pub icon: String,
	pub on_turn: Option<mlua::Function>,
	pub on_rest: Option<mlua::Function>,
	pub on_debuff: Option<mlua::Function>,
}

impl mlua::UserData for Component {}
