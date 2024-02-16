#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Piece {
    pub item: Item,
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Item {
    pub name: String,
}
