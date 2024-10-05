use esprit2::prelude::*;
use std::collections::VecDeque;
use std::sync::mpsc;

pub(crate) struct Dummy;

impl console::Handle for Dummy {
	fn send_message(&self, _message: console::Message) {}
}

#[derive(Debug, Clone)]
pub(crate) struct Handle {
	sender: mpsc::Sender<console::Message>,
}

impl console::Handle for Handle {
	fn send_message(&self, message: console::Message) {
		let _ = self.sender.send(message);
	}
}

#[derive(Debug)]
pub(crate) struct Console {
	pub(crate) handle: Handle,
	message_reciever: mpsc::Receiver<console::Message>,
	pub(crate) history: Vec<console::Message>,
	in_progress: VecDeque<usize>,
}

impl Default for Console {
	fn default() -> Self {
		let (sender, message_reciever) = mpsc::channel();
		Self {
			message_reciever,
			history: Vec::new(),
			in_progress: VecDeque::new(),
			handle: Handle { sender },
		}
	}
}

impl console::Handle for Console {
	fn send_message(&self, message: console::Message) {
		let _ = self.handle.sender.send(message);
	}
}

impl Console {
	pub(crate) fn update(&mut self, delta: f64) {
		for message in self.message_reciever.try_iter() {
			let is_dialogue = matches!(message.printer, console::MessagePrinter::Dialogue { .. });
			self.history.push(message);
			if is_dialogue {
				self.in_progress.push_back(self.history.len() - 1);
			}
		}

		let delta_progress = delta / 0.1;

		for i in &self.in_progress {
			let i = *i;
			let max_length = self.history[i].text.len() as f64;
			if let console::MessagePrinter::Dialogue {
				speaker: _,
				progress,
			} = &mut self.history[i].printer
			{
				let new_progress = *progress + delta_progress;
				if new_progress < max_length {
					*progress = new_progress;
				}
			}
		}

		while self.in_progress.front().is_some_and(|x| {
			if let console::MessagePrinter::Dialogue {
				speaker: _,
				progress,
			} = &self.history[*x].printer
			{
				self.history[*x].text.len() == (*progress as usize)
			} else {
				true
			}
		}) {
			self.in_progress.pop_front();
		}
	}
}
