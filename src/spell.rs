#[derive(Clone, Debug)]
pub struct Spell {
	pub name: Box<str>,
	pub description: Box<str>,
	pub icon: Box<str>,

	/// This is also the cost of the spell.
	pub level: u8,

	/// Whether or not the function can be casted by a given piece.
	pub castable: Option<mlua::Function>,
	/// Script to execute upon casting the spell.
	pub on_cast: mlua::Function,
	/// Script to return all possible spell actions.
	///
	/// Returns an array of `consider::Consideration`s for each possible usage of the spell.
	/// For an attack, this means potential targets.
	/// For a self-buff, this should roughly estimate the potential benefit of casting the spell.
	///
	/// When an on_consider script is about to be called, it's fed a list of characters that are potential targets for the spell.
	pub on_consider: Option<mlua::Function>,
	pub on_input: mlua::Function,
}

impl mlua::UserData for Spell {
	fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
		fields.add_field_method_get("level", |_, this| Ok(this.level));
		fields.add_field_method_get("on_consider", |_, this| Ok(this.on_consider.clone()));
	}
}
