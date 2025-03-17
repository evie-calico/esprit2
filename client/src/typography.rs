use crate::options::resource_directory;
use crate::Color;
use sdl3::iostream::IOStream;
use sdl3::ttf::Font;
use std::path::PathBuf;
use tracing::error;

pub(crate) struct Typography<'ttf_module, 'rwops> {
	pub(crate) normal: Font<'ttf_module, 'rwops>,
	pub(crate) annotation: Font<'ttf_module, 'rwops>,

	pub(crate) color: Color,
}

impl<'ttf_module> Typography<'ttf_module, '_> {
	/// # Errors
	///
	/// Returns an error if the font file could not be read.
	pub(crate) fn new(
		options: &Options,
		ttf_context: &'ttf_module sdl3::ttf::Sdl3TtfContext,
	) -> Self {
		let point_size = options.font_size;
		let annotation_size = f32::max(0.0, options.font_size - 2.0);

		let default_font_bytes = include_bytes!("res/FantasqueSansMNerdFontPropo-Regular.ttf");
		let open_font = |path: Option<&PathBuf>, size| {
			path.and_then(|path| {
				ttf_context
					.load_font(resource_directory().join(path), size)
					.map_err(|msg| error!("failed to open font {}: {msg}", path.display()))
					.ok()
			})
			.unwrap_or_else(|| {
				#[allow(clippy::unwrap_used, reason = "SDL")]
				ttf_context
					.load_font_from_iostream(
						IOStream::from_bytes(default_font_bytes).unwrap(),
						size,
					)
					.unwrap()
			})
		};

		Self {
			normal: open_font(options.font.as_ref(), point_size),
			annotation: open_font(options.font.as_ref(), annotation_size),
			color: options.font_color,
		}
	}
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct Options {
	pub(crate) font: Option<PathBuf>,
	pub(crate) font_size: f32,
	pub(crate) font_color: Color,
}

impl Default for Options {
	fn default() -> Self {
		Self {
			font: None,
			font_size: 18.0,
			font_color: (255, 255, 255, 255),
		}
	}
}
