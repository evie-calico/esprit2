use crate::prelude::*;
use paste::paste;
use std::sync::Arc;

#[derive(Clone, Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[archive(check_bytes)]
pub enum MessagePrinter {
	Console(Color),
	Dialogue { speaker: Arc<str>, progress: f64 },
	Combat(combat::Log),
}

#[derive(Clone, Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[archive(check_bytes)]
pub struct Message {
	pub text: String,
	pub printer: MessagePrinter,
}

macro_rules! console_colored_print {
	(normal) => {
		fn print(&self, text: String) {
			self.print_colored(text, Color::Normal);
		}
	};

	($which:ident) => {
		paste! {
			fn [<print_ $which>](&self, text: String) {
				self.print_colored(text, Color::[<$which:camel>]);
			}
		}
	};
}

macro_rules! handle_colored_print {
	(normal, $methods:ident) => {
		paste! {
			$methods.add_method("print", |_, this, text: String| {
				this.0.print_colored(text, Color::Normal);
				Ok(())
			});
		}
	};

	($which:ident, $methods:ident) => {
		paste! {
			$methods.add_method(concat!("print_", stringify!($which)), |_, this, text: String| {
				this.0.print_colored(text, Color::[<$which:camel>]);
				Ok(())
			});
		}
	};
}

macro_rules! impl_console {
	(
		$(impl $impl_colors:ident: $impl_value:expr,)+
		$(let $colors:ident: $value:expr,)+
	) => {
		paste! {
			#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
			#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
			#[archive(check_bytes)]
			pub enum Color {
				$([<$impl_colors:camel>],)*
			}
		}

		pub trait Handle {
			fn send_message(&self, message: Message);

			fn print_colored(&self, text: String, color: Color) {
				self.send_message(Message { text, printer: MessagePrinter::Console(color) })
			}

			fn say(&self, speaker: Arc<str>, text: String) {
				self.send_message(Message { text, printer: MessagePrinter::Dialogue { speaker, progress: 0.0 } })
			}

			fn combat_log(&self, text: String, log: combat::Log) {
				self.send_message(Message { text, printer: MessagePrinter::Combat(log) })
			}

			$(console_colored_print! { $impl_colors } )*
		}

		impl<T: Handle> Handle for &T {
			fn send_message(&self, message: Message) {
				(*self).send_message(message)
			}
		}

		pub struct LuaHandle<T: Handle>(pub T);

		impl<T: Handle> mlua::UserData for LuaHandle<T> {
			fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
				$(handle_colored_print! { $impl_colors, methods } )*
				methods.add_method("combat_log", |_, this, (text, log): (String, combat::Log)| {
					this.0.combat_log(text, log);
					Ok(())
				});

			}
		}
	};
}

impl_console! {
	impl normal: (255, 255, 255, 255),
	impl system: (100, 100, 100, 255),
	impl unimportant: (100, 100, 100, 255),
	impl defeat: (255, 128, 128, 255),
	impl danger: (255, 0, 0, 255),
	impl important: (255, 255, 0, 255),
	impl special: (0, 255, 0, 255),
	let combat: (255, 255, 128, 255),
}
