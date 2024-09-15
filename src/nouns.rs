use aho_corasick::AhoCorasick;
use std::sync::{Arc, LazyLock};

/// For dynamically addressing a character.
/// This should encompass almost every (dynamic) way of addressing someone or something.
#[derive(
	Clone,
	Debug,
	serde::Serialize,
	serde::Deserialize,
	alua::UserData,
	rkyv::Archive,
	rkyv::Serialize,
	rkyv::Deserialize,
)]
#[archive(check_bytes)]
pub struct Nouns {
	/// This is an `Arc<str>` rather than a `String` because it's very common to
	/// store a reference to a character's name (see `Console`).
	#[alua(as_lua = "string", get)]
	pub name: Arc<str>,
	/// If true, will be addressed as "Name", rather than "The name" or "A name".
	#[alua(get)]
	pub proper_name: bool,
	pub pronouns: Pronouns,
}

#[derive(
	Clone,
	Debug,
	Default,
	serde::Serialize,
	serde::Deserialize,
	rkyv::Archive,
	rkyv::Serialize,
	rkyv::Deserialize,
)]
#[archive(check_bytes)]
pub enum Pronouns {
	Female,
	Male,
	/// Neutral (they) is special because it necessitates "plural verbs".
	/// Even when used as a singular pronoun, verbs still treat "they" as plural.
	Neutral,
	#[default]
	Object,
}

pub trait StrExt {
	fn replace_prefixed_nouns(&self, nouns: &Nouns, prefix: &str) -> String;
	fn replace_nouns(&self, nouns: &Nouns) -> String;
}

impl<T: AsRef<str>> StrExt for T {
	fn replace_prefixed_nouns(&self, nouns: &Nouns, prefix: &str) -> String {
		self.as_ref().replace(prefix, "").replace_nouns(nouns)
	}

	fn replace_nouns(&self, nouns: &Nouns) -> String {
		static PRONOUN_TABLE: LazyLock<AhoCorasick> = LazyLock::new(|| {
			// "they" makes an ideal default because it has a unique word for all pronoun forms.
			// {are}/{Are} are provided for the neutral state form (it is cute / they are cute).
			// {s} is provided for the neutral verb form (it pounces / they pounce).
			AhoCorasick::new([
				"{they}",
				"{them}",
				"{their}",
				"{theirs}",
				"{themself}",
				"{are}",
				"{They}",
				"{Them}",
				"{Their}",
				"{Theirs}",
				"{Themself}",
				"{Are}",
				"{s}",
			])
			.expect("aho corasick table must be valid")
		});
		let replacements = match nouns.pronouns {
			Pronouns::Female => &[
				"she", "her", "her", "hers", "herself", "is", "She", "Her", "Her", "Hers",
				"Herself", "Is", "s",
			],
			Pronouns::Male => &[
				"he", "him", "his", "his", "himself", "is", "He", "Him", "His", "His", "Himself",
				"Is", "s",
			],
			// This pronoun does not represent plurality, but neutrality,
			// so "themself" is always the correct pronoun */
			Pronouns::Neutral => &[
				"they", "them", "their", "theirs", "themself", "are", "They", "Them", "Their",
				"Theirs", "Themself", "Are", "",
			],
			Pronouns::Object => &[
				"it", "its", "its", "its", "itself", "is", "It", "Its", "Its", "Its", "Itself",
				"Is", "s",
			],
		};
		let name = &[&*nouns.name];
		let capital_indirect_name = &["A ", &nouns.name];
		let indirect_name = &["a ", &nouns.name];
		let capital_address_name = &["The ", &nouns.name];
		let address_name = &["the ", &nouns.name];

		let mut s = PRONOUN_TABLE.replace_all(self.as_ref(), replacements);
		let mut i = 0;
		while let Some(start) = s[i..].find('{') {
			let start = i + start;
			let Some(end) = s[start..].find('}') else {
				break;
			};
			let end = start + end;
			let source = &s[(start + 1)..end];

			let replacement: &[&str] = match (nouns.proper_name, source) {
				(true, "Address" | "address" | "Indirect" | "indirect") => name,
				(false, "Address") => capital_address_name,
				(false, "address") => address_name,
				(false, "Indirect") => capital_indirect_name,
				(false, "indirect") => indirect_name,
				_ => {
					i = end;
					continue;
				}
			};

			let mut replacements = replacement.iter().rev();
			let first = replacements.next().expect("replacements must nt be empty");
			s.replace_range(start..end, first);
			// Convieniently, the name (longest entry) is first, meaning less copies when we need to fallback.
			for i in replacements {
				s.insert_str(start, i);
			}
			let new_end = s[start..]
				.find('}')
				.expect("replacements mus contain closing braces");
			s.remove(start + new_end);
			i = new_end;
		}
		s
	}
}
