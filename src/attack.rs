use crate::prelude::*;

/// Unlike spells, `Attack` is only for melee "bump attacks",
/// so their usage can be a lot simpler.
#[derive(
	Clone,
	Debug,
	serde::Serialize,
	serde::Deserialize,
	rkyv::Archive,
	rkyv::Serialize,
	rkyv::Deserialize,
)]
pub struct Attack {
	pub name: String,
	pub description: String,
	pub magnitude: Expression,
	pub on_input: Box<str>,
	pub on_use: Box<str>,
	pub on_consider: Option<Box<str>>,
	pub use_time: Aut,
}

impl mlua::UserData for Attack {
	fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
		fields.add_field_method_get("on_consider", |_, this| Ok(this.on_consider.clone()));
	}

	fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
		methods.add_method("magnitude", |_, this, user: character::Ref| {
			u32::evalv(&this.magnitude, &*user.borrow()).map_err(mlua::Error::external)
		});
	}
}
