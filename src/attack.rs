#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Attack {
	pub name: String,
	pub weight: u8,
	pub bonus: u32,
	pub messages: Messages,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Messages {
	// Special messages for "comically" low damage.
	pub low: Option<Vec<String>>,
	pub high: Vec<String>,
}
