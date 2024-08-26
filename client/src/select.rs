use esprit2::prelude::*;

#[derive(Clone, Debug)]
pub(crate) enum Point {
	Character(character::Ref),
	Exit(i32, i32),
}

/// Compiles all potential points of interest into a list.
pub(crate) fn assign_indicies(world: &world::Manager) -> Vec<Point> {
	world
		.characters
		.iter()
		.cloned()
		.map(Point::Character)
		.chain(world.current_floor.iter().filter_map(|(x, y, t)| {
			if t == floor::Tile::Exit {
				Some(Point::Exit(x as i32, y as i32))
			} else {
				None
			}
		}))
		.collect()
}
