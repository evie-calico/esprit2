use crate::character;

#[derive(Clone, Debug)]
pub struct Ability {
	/// How the ability is referred to in combat logs.
	pub name: Box<str>,
	/// A VERY brief note on the ability's usage requirements.
	///
	/// May be arbitrary and vague, but intended for spell SP costs.
	///
	/// Not used internally by the engine.
	/// This field exists purely for client convenience--future solutions may make it obsolete.
	pub usage: Option<Box<str>>,
	/// An extended description of the ability.
	///
	/// This may be excessively long and contain newline characters.
	///
	/// Like usage, this is not used by the engine and is provided only for client convenience.
	pub description: Option<Box<str>>,

	/// Whether or not this ability is currently usable.
	///
	/// on_use may assume that it will never be called when this function returns `false`;
	/// you should not check if the user's SP is >= cost in on_use when usable includes this check.
	pub usable: Option<mlua::Function>,
	/// Function to execute upon using the ability.
	///
	/// This is called with arguments generated either by on_consider or on_input
	/// (depending on the sentience of the piece's owner).
	pub on_use: mlua::Function,
	/// Function that returns all possible usages of this ability given a board state.
	///
	/// Returns an array of `consider::Consideration`s for each possible usage of the ability.
	/// For an attack, this means potential targets.
	/// For a self-buff, this should roughly estimate the potential benefit of applying it to a piece.
	///
	/// When an on_consider script is about to be called, it's fed a list of characters that are potential targets for the ability.
	///
	/// Note that, while being a field of `Ability`, on_consider is only ever read by Lua scripts.
	/// It would be possible (though potentially unwieldy?)
	/// to ability it to a lua-only resource table outside of the ability object.
	pub on_consider: Option<mlua::Function>,
	/// Function to collect user input.
	///
	/// This relies heavily on yields to create a "form".
	/// Potential input methods could be:
	/// - picking a location using a cursor/pointer
	/// - answering a yes/no question
	/// - picking a direction using arrow keys
	pub on_input: mlua::Function,
}

impl Ability {
	/// Shortcut for calling self.usable.
	///
	/// This function assumes that self.usable == None means the ability is always usable.
	pub fn usable(&self, user: character::Ref) -> mlua::Result<Option<Box<str>>> {
		self.usable
			.as_ref()
			.and_then(|x| x.call::<Option<Box<str>>>(user).transpose())
			.transpose()
	}
}

impl mlua::UserData for Ability {
	fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
		fields.add_field_method_get("on_consider", |_, this| Ok(this.on_consider.clone()));
	}
}
