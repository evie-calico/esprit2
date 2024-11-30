use std::collections::HashMap;

#[derive(
	PartialEq,
	Eq,
	Copy,
	Clone,
	Debug,
	Default,
	rkyv::Archive,
	rkyv::Serialize,
	rkyv::Deserialize,
	mlua::FromLua,
)]
// Keeping this very light is probably a good idea.
// Decorations, like statues and fountains and such, are sporadic and should be stored seperately.
// Don't go over 255 variants (reserve one for Option::None), and don't add members; they'll bloat the size of the map.
#[repr(u8)]
pub enum Tile {
	Floor,
	#[default]
	Wall,
	Exit,
}

impl mlua::UserData for Tile {
	fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
		methods.add_method("floor", |_, this, ()| Ok(matches!(this, Tile::Floor)));
		methods.add_method("wall", |_, this, ()| Ok(matches!(this, Tile::Wall)));
		methods.add_method("exit", |_, this, ()| Ok(matches!(this, Tile::Exit)));
	}
}

const CHUNK_SIZE: usize = 16;

#[derive(
	Clone, Copy, Debug, Eq, PartialEq, Hash, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize,
)]
#[rkyv(derive(Clone, Copy, Debug, Eq, PartialEq, Hash))]
pub struct ChunkId(i32, i32);

impl ChunkId {
	fn from_absolute(x: i32, y: i32) -> Self {
		Self(
			x.div_floor(CHUNK_SIZE as i32),
			y.div_floor(CHUNK_SIZE as i32),
		)
	}

	fn to_absolute(self, index: usize) -> (i32, i32) {
		(
			self.0 * CHUNK_SIZE as i32 + (index % CHUNK_SIZE) as i32,
			self.1 * CHUNK_SIZE as i32 + (index / CHUNK_SIZE) as i32,
		)
	}
}

#[derive(Clone, Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Chunk {
	map: [Option<Tile>; CHUNK_SIZE * CHUNK_SIZE],
}

impl Default for Chunk {
	fn default() -> Self {
		Self {
			map: [None; CHUNK_SIZE * CHUNK_SIZE],
		}
	}
}

#[derive(Clone, Debug, Default, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Floor {
	pub chunks: HashMap<ChunkId, Chunk>,
}

impl Floor {
	pub fn get(&self, x: i32, y: i32) -> Option<Tile> {
		let chunk_id = ChunkId::from_absolute(x, y);
		let chunk = self.chunks.get(&chunk_id)?;
		chunk.map[(x - chunk_id.0 * CHUNK_SIZE as i32
			+ (y - chunk_id.1 * CHUNK_SIZE as i32) * CHUNK_SIZE as i32) as usize]
	}

	pub fn get_mut(&mut self, x: i32, y: i32) -> &mut Option<Tile> {
		let chunk_id = ChunkId::from_absolute(x, y);
		let chunk = self.chunks.entry(chunk_id).or_default();
		&mut chunk.map[(x - chunk_id.0 * CHUNK_SIZE as i32
			+ (y - chunk_id.1 * CHUNK_SIZE as i32) * CHUNK_SIZE as i32) as usize]
	}

	/// This is not ordered!
	pub fn iter(&self) -> impl Iterator<Item = (i32, i32, Tile)> + '_ {
		self.chunks
			.iter()
			.flat_map(|(id, c)| {
				c.map.into_iter().enumerate().map(|(i, t)| {
					let (x, y) = id.to_absolute(i);
					(x, y, t)
				})
			})
			.filter_map(|(x, y, t)| t.map(|t| (x, y, t)))
	}
}
