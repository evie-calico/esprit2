use crate::prelude::*;
use options::resource_directory;
use sdl2::{rwops::RWops, ttf::Font};
use std::path::PathBuf;
use tracing::error;

pub struct Typography<'ttf_module, 'rwops> {
	pub normal: Font<'ttf_module, 'rwops>,
	pub annotation: Font<'ttf_module, 'rwops>,
	pub title: Font<'ttf_module, 'rwops>,

	pub color: Color,
}

impl<'ttf_module, 'rwops> Typography<'ttf_module, 'rwops> {
	/// # Errors
	///
	/// Returns an error if the font file could not be read.
	pub fn new(options: &Options, ttf_context: &'ttf_module sdl2::ttf::Sdl2TtfContext) -> Self {
		let point_size = options.font_size;
		let annotation_size = options.font_size.saturating_sub(2);
		let title_size = options.font_size.saturating_add(2);

		let default_font_bytes = include_bytes!("res/FantasqueSansMNerdFontPropo-Regular.ttf");
		let open_font = |path: Option<&PathBuf>, size| {
			path.and_then(|path| {
				ttf_context
					.load_font(resource_directory().join(path), size)
					.map_err(|msg| error!("failed to open font {}: {msg}", path.display()))
					.ok()
			})
			.unwrap_or_else(|| {
				ttf_context
					.load_font_from_rwops(RWops::from_bytes(default_font_bytes).unwrap(), size)
					.unwrap()
			})
		};

		Self {
			normal: open_font(options.font.as_ref(), point_size),
			annotation: open_font(options.font.as_ref(), annotation_size),
			title: open_font(options.font.as_ref(), title_size),
			color: options.font_color,
		}
	}
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Options {
	font: Option<PathBuf>,
	font_size: u16,
	font_color: Color,
}

impl Default for Options {
	fn default() -> Self {
		Self {
			font: None,
			font_size: 18,
			font_color: (255, 255, 255, 255),
		}
	}
}
