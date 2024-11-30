#[derive(Clone, Debug, Default, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Piece {
	pub item: Item,
	pub x: i32,
	pub y: i32,
}

#[derive(Clone, Debug, Default, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Item {
	pub name: String,
}
