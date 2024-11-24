use pest::pratt_parser::PrattParser;
use pest::Parser;
use rand::Rng;
// TODO: Return errors instead of printing them.
use tracing::error;

pub type Integer = i64;

#[derive(Clone, Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub enum Operation {
	Integer(Integer),
	Variable(usize, usize),

	// Self-referrential
	Add(usize, usize),
	Sub(usize, usize),
	Mul(usize, usize),
	Div(usize, usize),
	// Constant operations
	AddC(usize, Integer),
	SubC(usize, Integer),
	MulC(usize, Integer),
	DivC(usize, Integer),
	Roll(Integer, Integer),
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("cannot evaluate variable \"{0}\": no variables defined")]
	NoVariables(String),
	#[error("variable \"{0}\" not defined")]
	MissingVariable(String),
	#[error("result ({0}) out of range for {1}")]
	OutOfRange(Integer, &'static str),
}

impl Operation {
	fn eval(&self, equation: &Expression, variables: &impl Variables) -> Result<Integer, Error> {
		let get_leaf = |i: usize| {
			equation
				.leaves
				.get(i)
				.expect("invalid leaf indices")
				.eval(equation, variables)
		};

		match self {
			Operation::Integer(i) => Ok(*i),
			Operation::Variable(from, to) => variables.get(&equation.source[*from..*to]),
			Operation::Roll(amount, die) => {
				Ok((0..*amount).fold(0, |a, _| a + rand::thread_rng().gen_range(1..=*die)))
			}
			Operation::Add(a, b) => Ok(get_leaf(*a)? + get_leaf(*b)?),
			Operation::Sub(a, b) => Ok(get_leaf(*a)? - get_leaf(*b)?),
			Operation::Mul(a, b) => Ok(get_leaf(*a)? * get_leaf(*b)?),
			Operation::Div(a, b) => Ok(get_leaf(*a)? / get_leaf(*b)?),
			Operation::AddC(x, i) => Ok(get_leaf(*x)? + i),
			Operation::SubC(x, i) => Ok(get_leaf(*x)? - i),
			Operation::MulC(x, i) => Ok(get_leaf(*x)? * i),
			Operation::DivC(x, i) => Ok(get_leaf(*x)? / i),
		}
	}
}

pub trait Variables {
	/// # Errors
	///
	/// Should return `Err(expression::Error::MissingVariable(s)` if a variable is not defined.
	fn get(&self, s: &str) -> Result<Integer, Error>;
}

impl Variables for () {
	fn get(&self, s: &str) -> Result<Integer, Error> {
		Err(Error::NoVariables(s.into()))
	}
}

impl Variables for Integer {
	fn get(&self, _: &str) -> Result<Integer, Error> {
		Ok(*self)
	}
}

impl Variables for mlua::Table {
	fn get(&self, s: &str) -> Result<Integer, Error> {
		self.get::<Integer>(s)
			.map_err(|_| Error::MissingVariable(s.into()))
	}
}

impl Variables for mlua::AnyUserData {
	fn get(&self, s: &str) -> Result<Integer, Error> {
		<Self as mlua::ObjectLike>::get::<Integer>(self, s)
			.map_err(|_| Error::MissingVariable(s.into()))
	}
}

#[derive(Clone, Debug, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, mlua::FromLua)]
pub struct Expression {
	pub source: String,
	pub root: Operation,
	pub leaves: Vec<Operation>,
}

impl Default for Expression {
	fn default() -> Self {
		Self {
			source: String::from("0"),
			root: Operation::Integer(0),
			leaves: Vec::new(),
		}
	}
}

impl mlua::UserData for Expression {
	fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
		methods.add_meta_method(
			"__call",
			|_, this, args: mlua::Either<mlua::Table, mlua::AnyUserData>| match args {
				mlua::Either::Left(table) => {
					mlua::Integer::evalv(this, &table).map_err(mlua::Error::external)
				}
				mlua::Either::Right(userdata) => {
					mlua::Integer::evalv(this, &userdata).map_err(mlua::Error::external)
				}
			},
		);
	}
}

impl TryFrom<String> for Expression {
	type Error = pest::error::Error<Rule>;

	fn try_from(source: String) -> Result<Self, Self::Error> {
		let pairs = OperationParser::parse(Rule::equation, &source)?
			.next()
			.expect("pest returned no pairs")
			.into_inner();

		let mut leaves = Vec::new();

		let mut add_leaf = |leaf: Operation| -> usize {
			leaves.push(leaf);
			leaves.len() - 1
		};

		let root =
			pratt_parser()
				.map_primary(|primary| match primary.as_rule() {
					Rule::integer => Operation::Integer(
						primary
							.as_str()
							.parse()
							.expect("parser must return valid integer characters"),
					),
					Rule::identifier => {
						let span = primary.as_span();
						Operation::Variable(span.start(), span.end())
					}
					Rule::roll => {
						let (amount, die) = primary
							.as_str()
							.split_once('d')
							.expect("parser must return a string containing a 'd'");
						Operation::Roll(
							amount
								.parse()
								.expect("parser must return valid integer characters"),
							die.parse()
								.expect("parser must return valid integer characters"),
						)
					}
					rule => unreachable!(
						"Expr::parse expected terminal value, found {rule:?} ({})",
						primary.as_str()
					),
				})
				.map_infix(|lhs, op, rhs| match (lhs, op.as_rule(), rhs) {
					// Constant resolution
					(Operation::Integer(i), Rule::add, x)
					| (x, Rule::add, Operation::Integer(i)) => Operation::AddC(add_leaf(x), i),
					(Operation::Integer(i), Rule::sub, x)
					| (x, Rule::sub, Operation::Integer(i)) => Operation::SubC(add_leaf(x), i),
					(Operation::Integer(i), Rule::mul, x)
					| (x, Rule::mul, Operation::Integer(i)) => Operation::MulC(add_leaf(x), i),
					(Operation::Integer(i), Rule::div, x)
					| (x, Rule::div, Operation::Integer(i)) => Operation::DivC(add_leaf(x), i),
					(lhs, Rule::add, rhs) => Operation::Add(add_leaf(lhs), add_leaf(rhs)),
					(lhs, Rule::sub, rhs) => Operation::Sub(add_leaf(lhs), add_leaf(rhs)),
					(lhs, Rule::mul, rhs) => Operation::Mul(add_leaf(lhs), add_leaf(rhs)),
					(lhs, Rule::div, rhs) => Operation::Div(add_leaf(lhs), add_leaf(rhs)),
					rule => unreachable!("Expr::parse expected infix operation, found {rule:?}"),
				})
				.parse(pairs);
		Ok(Self {
			source,
			root,
			leaves,
		})
	}
}

pub trait Evaluate<'variables>: Sized {
	fn eval(expression: &Expression) -> Result<Self, Error> {
		Self::evalv(expression, &())
	}

	fn evalv(expression: &Expression, variables: &'variables impl Variables)
		-> Result<Self, Error>;
}

macro_rules! impl_int {
	($type:ident) => {
		impl<'variables> Evaluate<'variables> for $type {
			fn evalv(
				expression: &Expression,
				variables: &'variables impl Variables,
			) -> Result<Self, Error> {
				let value = expression.root.eval(expression, variables)?;
				$type::try_from(value).map_err(|_| Error::OutOfRange(value, stringify!($type)))
			}
		}
	};
}

impl_int!(u8);
impl_int!(u16);
impl_int!(u32);
impl_int!(u64);
impl_int!(u128);
impl_int!(i8);
impl_int!(i16);
impl_int!(i32);
impl_int!(i64);
impl_int!(i128);

impl serde::Serialize for Expression {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		serializer.serialize_str(&self.source)
	}
}

struct ExpressionVisitor;

impl<'de> serde::de::Visitor<'de> for ExpressionVisitor {
	type Value = String;

	fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
		formatter.write_str("a string containing an expression")
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

impl<'de> serde::Deserialize<'de> for Expression {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		use serde::de::Error;
		Expression::try_from(deserializer.deserialize_string(ExpressionVisitor)?)
			.map_err(D::Error::custom)
	}
}

#[derive(pest_derive::Parser)]
#[grammar = "expression.pest"]
struct OperationParser;

fn pratt_parser() -> &'static PrattParser<Rule> {
	use pest::pratt_parser::{Assoc::*, Op};
	use std::sync::OnceLock;
	use Rule::*;

	static PRATT_PARSER: OnceLock<PrattParser<Rule>> = OnceLock::new();
	PRATT_PARSER.get_or_init(|| {
		// Precedence is defined lowest to highest
		PrattParser::new()
			// Addition and subtract have equal precedence
			.op(Op::infix(add, Left) | Op::infix(sub, Left))
			.op(Op::infix(mul, Left) | Op::infix(div, Left))
	})
}
