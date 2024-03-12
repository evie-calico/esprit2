use aho_corasick::AhoCorasick;
use std::rc::Rc;
use std::sync::LazyLock;

/// For dynamically addressing a character.
/// This should encompass almost every (dynamic) way of addressing someone or something.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Nouns {
	/// This is an `Rc<str>` rather than a `String` because it's very common to
	/// store a reference to a character's name (see `Console`).
	pub name: Rc<str>,
	/// If true, will be addressed as "Name", rather than "The name" or "A name".
	pub proper_name: bool,
	pub pronouns: Pronouns,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
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
			AhoCorasick::new([
				"{they}", "{them}", "{their}", "{theirs}", "{are}", "{They}", "{Them}", "{Their}",
				"{Theirs}", "{Are}",
			])
			.unwrap()
		});
		let replacements = match nouns.pronouns {
			Pronouns::Female => &[
				"she", "her", "her", "hers", "is", "She", "Her", "Her", "Hers", "Is",
			],
			Pronouns::Male => &[
				"he", "him", "his", "his", "is", "He", "Him", "His", "His", "Is",
			],
			Pronouns::Neutral => &[
				"they", "them", "their", "theirs", "are", "They", "Them", "Their", "Theirs", "Are",
			],
			Pronouns::Object => &[
				"it", "its", "its", "its", "is", "It", "Its", "Its", "Its", "Is",
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
			let first = replacements.next().unwrap();
			s.replace_range(start..end, first);
			// Convieniently, the name (longest entry) is first, meaning less copies when we need to fallback.
			for i in replacements {
				s.insert_str(start, i);
			}
			let new_end = s[start..].find('}').unwrap();
			s.remove(start + new_end);
			i = new_end;
		}
		s
	}
}
