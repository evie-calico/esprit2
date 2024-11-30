use crate::prelude::*;
use paste::paste;

#[derive(Clone, Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub enum MessagePrinter {
	Console(Color),
	Dialogue { speaker: Box<str>, progress: f64 },
	Combat(combat::Log),
}

#[derive(Clone, Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Message {
	pub text: Box<str>,
	pub printer: MessagePrinter,
}

macro_rules! console_colored_print {
	(normal) => {
		fn print(&self, text: impl Into<Box<str>>) {
			self.print_colored(text, Color::Normal);
		}
	};

	($which:ident) => {
		paste! {
			fn [<print_ $which>](&self, text: impl Into<Box<str>>) {
				self.print_colored(text, Color::[<$which:camel>]);
			}
		}
	};
}

macro_rules! handle_colored_print {
	(normal, $methods:ident) => {
		paste! {
			$methods.add_method("print", |_, this, text: mlua::String| {
				this.0.print_colored(text.to_str()?.as_ref(), Color::Normal);
				Ok(())
			});
		}
	};

	($which:ident, $methods:ident) => {
		paste! {
			$methods.add_method(concat!("print_", stringify!($which)), |_, this, text: mlua::String| {
				this.0.print_colored(text.to_str()?.as_ref(), Color::[<$which:camel>]);
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
			#[derive(Clone, Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
			pub enum Color {
				$([<$impl_colors:camel>],)*
			}
		}

		pub trait Handle {
			fn send_message(&self, message: Message);

			fn print_colored(&self, text: impl Into<Box<str>>, color: Color) {
				self.send_message(Message { text: text.into(), printer: MessagePrinter::Console(color) })
			}

			fn say(&self, speaker: impl Into<Box<str>>, text: impl Into<Box<str>>) {
				self.send_message(Message { text: text.into(), printer: MessagePrinter::Dialogue { speaker: speaker.into(), progress: 0.0 } })
			}

			fn combat_log(&self, text: impl Into<Box<str>>, log: combat::Log) {
				self.send_message(Message { text: text.into(), printer: MessagePrinter::Combat(log) })
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
			fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
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
