//! "A* & co has been overdone a million times."

use crate::prelude::*;
use std::collections::VecDeque;

/// Representation of distance from a target.
///
/// Keep this reasonably small to avoid allocating lots of memory,
/// without getting so small that the map breaks at long distances.
/// (A u8 would be limited to about 20 turns of unencumbered movement)
type Distance = u16;

pub const UNEXPLORED: u16 = u16::MAX;
pub const IMPASSABLE: u16 = u16::MAX - 1;

/// A grid of distances from an arbitrary number of targets.
#[derive(Clone, Debug)]
pub struct DijkstraMap {
	width: usize,
	grid: Box<[Distance]>,
	frontier: VecDeque<(usize, usize)>,
}

/// Grid access
impl DijkstraMap {
	fn get(&self, x: usize, y: usize) -> Option<Distance> {
		if x >= self.width {
			return None;
		}
		self.grid.get(x + y * self.width).copied()
	}

	fn get_mut(&mut self, x: usize, y: usize) -> Option<&mut Distance> {
		if x >= self.width {
			return None;
		}
		self.grid.get_mut(x + y * self.width)
	}
}

/// Construction
impl DijkstraMap {
	fn explore_tile(&mut self, x: usize, y: usize, distance: Distance) {
		if let Some(tile) = self.get_mut(x, y) {
			*tile = distance;
			self.frontier.push_back((x, y));
		}
	}

	pub fn target(width: usize, height: usize, targets: &[(i32, i32)]) -> Self {
		let mut grid = Box::new_uninit_slice(width * height);
		std::mem::MaybeUninit::fill(&mut grid, UNEXPLORED);
		let grid = unsafe { grid.assume_init() };

		let mut map = Self {
			width,
			grid,
			frontier: VecDeque::new(),
		};
		for (x, y) in targets.iter().cloned() {
			if let Ok(x) = x.try_into()
				&& let Ok(y) = y.try_into()
			{
				map.explore_tile(x, y, 0);
			}
		}
		map
	}

	pub fn explore(
		&mut self,
		x: usize,
		y: usize,
		evaluate_tile: impl Fn(usize, usize, Distance) -> Distance,
	) {
		// TODO: better heuristics for searching frontier
		while let Some(next) = self.frontier.pop_front() {
			let base_distance = self
				.get(next.0, next.1)
				.expect("frontiers should never be out of bounds");
			for direction in OrdDir::all().map(OrdDir::as_offset) {
				// Shorten any nearby paths.
				// Remember that IMPASSIBLE and UNEXPLORED are represented by very large integers.
				if let Ok(ax) = (next.0 as i32 + direction.0).try_into()
					&& let Ok(ay) = (next.1 as i32 + direction.1).try_into()
					&& let Some(tile) = self.get(ax, ay)
					&& tile != IMPASSABLE
				{
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
}

/// Usage
impl DijkstraMap {
	pub fn step(&mut self, x: i32, y: i32) -> Option<OrdDir> {
		OrdDir::all()
			.fold(None, |a: Option<(OrdDir, Distance)>, direction: OrdDir| {
				let (xoff, yoff) = direction.as_offset();
				if let Some(x) = (x + xoff).try_into().ok()
					&& let Some(y) = (y + yoff).try_into().ok()
					&& let Some(tile) = self.get(x, y)
					&& tile != IMPASSABLE
					&& tile != UNEXPLORED
					&& !a.is_some_and(|a| a.1 < tile)
				{
					Some((direction, tile))
				} else {
					a
				}
			})
			.map(|x| x.0)
	}
}

impl std::fmt::Display for DijkstraMap {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		for (i, tile) in self.grid.iter().copied().enumerate() {
			let repr = match tile {
				0..=9 => char::from_digit(tile as u32, 10).expect("tile must be in digit range"),
				10..36 => char::from_u32(tile as u32 - 10 + 'a' as u32)
					.expect("tile must be within ascii latin letter range"),
				UNEXPLORED => '?',
				IMPASSABLE => '.',
				_ => '!',
			};
			write!(f, "{repr}")?;
			if i != self.grid.len() - 1 && (i + 1) % self.width == 0 {
				writeln!(f)?;
			}
		}
		Ok(())
	}
}
