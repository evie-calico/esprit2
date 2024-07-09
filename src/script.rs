use crate::prelude::*;
use std::fs;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", content = "source")]
pub enum MaybeInline {
	Inline(String),
	Path(Script),
}

impl MaybeInline {
	pub fn name(&self, parent: &str) -> String {
		match self {
			MaybeInline::Inline(_) => format!("{parent} (inline)"),
			MaybeInline::Path(Script { path, .. }) => path.clone(),
		}
	}
	pub fn contents(&self) -> &str {
		match self {
			MaybeInline::Inline(s) => s,
			MaybeInline::Path(expression) => &expression.contents,
		}
	}
}

#[derive(Clone, Debug)]
pub struct Script {
	pub path: String,
	pub contents: String,
}

impl serde::Serialize for Script {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		serializer.serialize_str(&self.path)
	}
}

struct ScriptVisitor;

impl<'de> serde::de::Visitor<'de> for ScriptVisitor {
	type Value = String;

	fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
		formatter.write_str("the path to a Lua script")
	}

	fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
	where
		E: serde::de::Error,
	{
		Ok(value)
	}

	fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
	where
		E: serde::de::Error,
	{
		Ok(value.to_string())
	}
}

impl<'de> serde::Deserialize<'de> for Script {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		use serde::de::Error;
		let path = deserializer.deserialize_string(ScriptVisitor)?;
		let contents = fs::read_to_string(options::resource_directory().join(&path))
			.map_err(D::Error::custom)?;
		Ok(Script { path, contents })
	}
}
