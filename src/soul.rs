use crate::prelude::*;

pub struct Soul {
	pub color: Color,
	pub x: f32,
	pub y: f32,
	/// The souls avoid the edges of walls a bit.
	/// These variables slightly offset what they consider to be the center.
	x_offset: f32,
	y_offset: f32,
	/// How fast the soul is moving towards its target position.
	speed: f32,
	/// Seconds
	///
	/// Once this hits 0, speed will be re-calculated.
	speed_timer: f32,
	path_position: f32,
	path_speed: f32,
}

impl Soul {
	pub fn new(color: Color) -> Self {
		let mut rng = rand::thread_rng();
		let mut new = Self {
			color,
			x: rng.gen(),
			y: rng.gen(),

			// None of these fields are final; the following function will fill them in.
			x_offset: 0.0,
			y_offset: 0.0,
			speed: 0.0,
			speed_timer: 0.0,
			path_position: 0.0,
			path_speed: 0.0,
		};
		new.speed_timer_timeout();
		new
	}

	pub fn speed_timer_timeout(&mut self) {
		let mut rng = rand::thread_rng();
		self.speed = rng.gen_range(0.5..1.0);
		self.x_offset = rng.gen();
		self.y_offset = rng.gen();
		if rng.gen_range(0..100) < 20 {
			self.path_position = rng.gen_range(0.0..100.0);
		}
		self.path_speed = rng.gen_range(0.5..1.0);
		self.speed_timer = rng.gen_range(3.0..8.0);
	}

	pub fn tick(&mut self, delta: f32) {
		let progress = self.path_position;
		let squish = |pos, by, off| pos * by + (1.0 - by) * off;
		let target_position = (
			squish(progress.cos() / 2.0 + 0.5, 0.8, self.x_offset),
			squish(progress.sin() / 2.0 + 0.5, 0.5, self.y_offset),
		);

		// Move
		let x_diff = (self.x - target_position.0).abs() * delta * self.speed;
		if self.x < target_position.0 {
			self.x += x_diff;
		} else if self.x > target_position.0 {
			self.x -= x_diff;
		}

		let y_diff = (self.y - target_position.0).abs() * delta * self.speed;
		if self.y < target_position.1 {
			self.y += y_diff;
		} else if self.y > target_position.1 {
			self.y -= y_diff;
		}

		// Update stats.
		self.path_position += delta * self.path_speed;
		self.speed_timer -= delta;
		if self.speed_timer <= 0.0 {
			self.speed_timer_timeout();
		}
	}
}
