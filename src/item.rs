#[derive(
	Clone,
	Debug,
	Default,
	serde::Serialize,
	serde::Deserialize,
	rkyv::Archive,
	rkyv::Serialize,
	rkyv::Deserialize,
)]
#[archive(check_bytes)]
pub struct Piece {
	pub item: Item,
	pub x: i32,
	pub y: i32,
}

#[derive(
	Clone,
	Debug,
	Default,
	serde::Serialize,
	serde::Deserialize,
	rkyv::Archive,
	rkyv::Serialize,
	rkyv::Deserialize,
)]
#[archive(check_bytes)]
pub struct Item {
	pub name: String,
}
