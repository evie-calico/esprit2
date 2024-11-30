//! "A* & co has been overdone a million times."

use crate::prelude::*;
use std::collections::HashMap;

/// Representation of distance from a target.
///
/// Keep this reasonably small to avoid allocating lots of memory,
/// without getting so small that the map breaks at long distances.
/// (A u8 would be limited to about 20 turns of unencumbered movement)
type Distance = u16;

pub const UNEXPLORED: u16 = u16::MAX;
pub const IMPASSABLE: u16 = u16::MAX - 1;

const CHUNK_SIZE: usize = 16;

#[derive(
	Clone, Copy, Debug, Eq, PartialEq, Hash, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize,
)]
#[rkyv(derive(Clone, Copy, Debug, Eq, PartialEq, Hash))]
struct ChunkId(i32, i32);

impl ChunkId {
	fn from_absolute(x: i32, y: i32) -> Self {
		Self(
			x.div_floor(CHUNK_SIZE as i32),
			y.div_floor(CHUNK_SIZE as i32),
		)
	}
}

#[derive(Clone, Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
struct Chunk {
	map: [Distance; CHUNK_SIZE * CHUNK_SIZE],
}

impl Default for Chunk {
	fn default() -> Self {
		Self {
			map: [UNEXPLORED; CHUNK_SIZE * CHUNK_SIZE],
		}
	}
}

#[derive(Clone, Debug, Default, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Floor {
	chunks: HashMap<ChunkId, Chunk>,
	frontier: Vec<(i32, i32)>,
}

impl Floor {
	#[inline(always)]
	pub fn get(&self, x: i32, y: i32) -> Distance {
		let chunk_id = ChunkId::from_absolute(x, y);
		if let Some(chunk) = self.chunks.get(&chunk_id) {
			chunk.map[(x - chunk_id.0 * CHUNK_SIZE as i32
				+ (y - chunk_id.1 * CHUNK_SIZE as i32) * CHUNK_SIZE as i32) as usize]
		} else {
			UNEXPLORED
		}
	}

	#[inline(always)]
	pub fn get_mut(&mut self, x: i32, y: i32) -> &mut Distance {
		let chunk_id = ChunkId::from_absolute(x, y);
		let chunk = self.chunks.entry(chunk_id).or_default();
		&mut chunk.map[(x - chunk_id.0 * CHUNK_SIZE as i32
			+ (y - chunk_id.1 * CHUNK_SIZE as i32) * CHUNK_SIZE as i32) as usize]
	}
}

/// Construction
impl Floor {
	#[inline(always)]
	fn explore_tile(&mut self, x: i32, y: i32, distance: Distance) {
		*self.get_mut(x, y) = distance;
		if distance != IMPASSABLE {
			self.frontier.push((x, y));
		}
	}

	pub fn target(targets: &[(i32, i32)]) -> Self {
		let mut map = Self::default();
		for (x, y) in targets.iter().cloned() {
			map.explore_tile(x, y, 0);
		}
		map
	}

	pub fn explore(
		&mut self,
		x: i32,
		y: i32,
		evaluate_tile: impl Fn(i32, i32, Distance) -> Distance,
	) {
		loop {
			// TODO: Use a better sorting algorithm since this is sorted until pushed to.
			self.frontier
				.sort_unstable_by(|a, b| (a.0 - x + a.1 - y).cmp(&(b.0 - x + b.1 - y)).reverse());

			let Some(next) = self.frontier.pop() else {
				break;
			};

			let base_distance = self.get(next.0, next.1);
			for direction in OrdDir::all().map(OrdDir::as_offset) {
				// Shorten any nearby paths.
				// Remember that IMPASSIBLE and UNEXPLORED are represented by very large integers.
				let ax = next.0 + direction.0;
				let ay = next.1 + direction.1;
				let tile = self.get(ax, ay);
				if tile != IMPASSABLE {
					let distance = evaluate_tile(ax, ay, base_distance);
					if distance < tile {
						self.explore_tile(ax, ay, distance);
					}
					if ax == x && ay == y {
						return;
					}
				}
			}
		}
	}

	pub fn step(&mut self, x: i32, y: i32) -> Option<OrdDir> {
		OrdDir::all()
			.fold(None, |a: Option<(OrdDir, Distance)>, direction: OrdDir| {
				let (xoff, yoff) = direction.as_offset();
				let x = x + xoff;
				let y = y + yoff;
				let tile = self.get(x, y);
				if tile != IMPASSABLE && tile != UNEXPLORED && a.is_none_or(|a| a.1 >= tile) {
					Some((direction, tile))
				} else {
					a
				}
			})
			.map(|x| x.0)
	}
}
