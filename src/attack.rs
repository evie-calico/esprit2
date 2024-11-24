use crate::prelude::*;

/// Unlike spells, `Attack` is only for melee "bump attacks",
/// so their usage can be a lot simpler.
#[derive(Clone, Debug)]
pub struct Attack {
	pub name: String,
	pub description: String,
	pub magnitude: Expression,
	pub on_input: mlua::Function,
	pub on_use: mlua::Function,
	pub on_consider: Option<mlua::Function>,
	pub use_time: Aut,
}

impl mlua::UserData for Attack {
	fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
		fields.add_field_method_get("on_consider", |_, this| Ok(this.on_consider.clone()));
		fields.add_field_method_get("use_time", |_, this| Ok(this.use_time));
		fields.add_field_method_get("magnitude", |_, this| Ok(this.magnitude.clone()));
	}
}
