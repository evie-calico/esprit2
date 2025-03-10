#[derive(Clone, Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[rkyv(serialize_bounds(
    __S: rkyv::ser::Writer + rkyv::ser::Allocator,
    __S::Error: rkyv::rancor::Source,
))]
#[rkyv(deserialize_bounds(__D::Error: rkyv::rancor::Source))]
#[rkyv(bytecheck(
    bounds(
        __C: rkyv::validation::ArchiveContext,
    )
))]
pub enum Value {
	Unit,
	Boolean(bool),
	Integer(mlua::Integer),
	Number(mlua::Number),
	String(Box<str>),
	Table(#[rkyv(omit_bounds)] Box<[(Value, Value)]>),
	/// This variant is a shortcut for a Lua "array" without keys.
	OrderedTable(#[rkyv(omit_bounds)] Box<[Value]>),
}

impl Value {
	pub fn as_lua(&self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
		Ok(match self {
			// This is distinct from nil: lightuserdata are considered truthy but have no other functionality (similar to `true`).
			// A `nil` value would delete the key it's associated with when converted to lua,
			// so mlua's NULL constant provides a workaround.
			Value::Unit => mlua::Value::NULL,
			Value::Boolean(i) => mlua::Value::Boolean(*i),
			Value::Integer(i) => mlua::Value::Integer(*i),
			Value::Number(i) => mlua::Value::Number(*i),
			Value::String(i) => mlua::Value::String(lua.create_string(&**i)?),
			Value::Table(i) => {
				let table = lua.create_table()?;
				for (k, v) in i {
					table.set(k.as_lua(lua)?, v.as_lua(lua)?)?;
				}
				mlua::Value::Table(table)
			}
			Value::OrderedTable(i) => {
				let table = lua.create_table()?;
				let mut k = 0;
				#[expect(
					clippy::explicit_counter_loop,
					reason = "https://doc.rust-lang.org/stable/edition-guide/rust-2024/intoiterator-box-slice.html"
				)]
				for v in i {
					table.set(k + 1, v.as_lua(lua)?)?;
					k += 1;
				}
				mlua::Value::Table(table)
			}
		})
	}
}

impl mlua::FromLua for Value {
	fn from_lua(value: mlua::Value, _lua: &mlua::Lua) -> mlua::Result<Self> {
		// I can't match on mlua::Value::NULL directly so this is a reconstructed version of the LightUserData it wraps.
		const NULL: mlua::LightUserData = mlua::LightUserData(std::ptr::null_mut());
		match value {
			mlua::Value::Nil | mlua::Value::LightUserData(NULL) => Ok(Value::Unit),
			mlua::Value::Boolean(i) => Ok(Value::Boolean(i)),
			mlua::Value::Integer(i) => Ok(Value::Integer(i)),
			mlua::Value::Number(i) => Ok(Value::Number(i)),
			mlua::Value::String(i) => Ok(Value::String(i.to_str()?.as_ref().into())),
			mlua::Value::Table(i) => {
				let mut integer_only = true;
				i.for_each(|k: mlua::Value, _v: mlua::Value| {
					if !k.is_integer() {
						integer_only = false;
					}
					Ok(())
				})?;
				if integer_only {
					Ok(Value::OrderedTable(
						i.sequence_values::<Self>()
							.collect::<mlua::Result<Box<[Self]>>>()?,
					))
				} else {
					Ok(Value::Table(
						i.pairs::<Self, Self>()
							.collect::<mlua::Result<Box<[(Self, Self)]>>>()?,
					))
				}
			}
			_ => Err(mlua::Error::runtime(format!(
				"type \"{}\" cannot be converted to an esprit Value",
				value.type_name()
			))),
		}
	}
}

impl mlua::IntoLua for Value {
	fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
		self.as_lua(lua)
	}
}
