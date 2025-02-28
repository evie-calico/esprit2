#[derive(Clone, Debug)]
pub struct Component {
	pub name: String,
	pub icon: Option<String>,
	/// If `true`, the component should be displayed to the user on stat screens.
	pub visible: bool,
	/// Called any time the component is attached to a piece.
	///
	/// This function is called after the new component and value have been added,
	/// and on_attach allowed to remove for any reason.
	/// The previous state is passed as an argument,
	/// which can be used to manipulate the respresentation as desired.
	pub on_attach: Option<mlua::Function>,
	/// Called any time the component is detached from a piece.
	///
	/// This is called after the component is removed,
	/// and on_detach is allowed to reattach the component (or attach other ones!).
	/// The previous value of the component is passed to this function.
	///
	/// detach may provide an "annotation", similar to attach.
	/// This does nothing, but gets passed down to on_detach.
	pub on_detach: Option<mlua::Function>,
	/// Called any time a turn is taken.
	///
	/// Recieves the piece and the time the turn took as arguments.
	pub on_turn: Option<mlua::Function>,
	/// Called any time a piece "rests".
	///
	/// What this means is a little unclear but i previously used it whenever an exit was taken.
	pub on_rest: Option<mlua::Function>,
	/// Used to determine any deductions that need to be applied to the piece's stats.
	///
	/// Recieves only the component value as an argument, not the piece.
	pub on_debuff: Option<mlua::Function>,
}

impl mlua::UserData for Component {}
