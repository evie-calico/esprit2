use pest::iterators::Pairs;
use pest::pratt_parser::PrattParser;
use pest::Parser;
use std::ascii;
use std::str::FromStr;

#[derive(Clone, Debug)]
enum Expression {
	Integer(u32),
	Variable(usize, usize),
	Add(usize, usize),
	Sub(usize, usize),
	Mul(usize, usize),
	Div(usize, usize),
}

#[derive(Debug, thiserror::Error)]
pub enum Error<'expression> {
	#[error("cannot evaluate variable \"{0}\": no variables defined")]
	NoVariables(&'expression str),
	#[error("variable \"{0}\" not defined")]
	MissingVariable(&'expression str),
}

impl Expression {
	fn eval<'expression>(
		&self,
		equation: &'expression Equation,
		variables: &'expression impl Variables,
	) -> Result<u32, Error<'expression>> {
		match self {
			Expression::Integer(i) => Ok(*i),
			Expression::Variable(from, to) => {
				variables.get(equation.identifiers[*from..*to].as_str())
			}
			Expression::Add(a, b) => {
				Ok(equation.leaves.get(*a).unwrap().eval(equation, variables)?
					+ equation.leaves.get(*b).unwrap().eval(equation, variables)?)
			}
			Expression::Sub(a, b) => {
				Ok(equation.leaves.get(*a).unwrap().eval(equation, variables)?
					- equation.leaves.get(*b).unwrap().eval(equation, variables)?)
			}
			Expression::Mul(a, b) => {
				Ok(equation.leaves.get(*a).unwrap().eval(equation, variables)?
					* equation.leaves.get(*b).unwrap().eval(equation, variables)?)
			}
			Expression::Div(a, b) => {
				Ok(equation.leaves.get(*a).unwrap().eval(equation, variables)?
					/ equation.leaves.get(*b).unwrap().eval(equation, variables)?)
			}
		}
	}
}

pub trait Variables {
	/// # Errors
	///
	/// Should return `Err(expression::Error::MissingVariable(s)` if a variable is not defined.
	fn get<'expression>(&self, s: &'expression str) -> Result<u32, Error<'expression>>;
}

impl Variables for () {
	fn get<'expression>(&self, s: &'expression str) -> Result<u32, Error<'expression>> {
		Err(Error::NoVariables(s))
	}
}

#[derive(Clone, Debug)]
pub struct Equation {
	root: Expression,
	leaves: Vec<Expression>,
	identifiers: Vec<ascii::Char>,
}

impl FromStr for Equation {
	type Err = pest::error::Error<Rule>;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(Equation::parse_pairs(
			ExpressionParser::parse(Rule::equation, s)?
				.next()
				.unwrap()
				.into_inner(),
		))
	}
}

impl Equation {
	/// # Errors
	///
	/// Returns an error if the variable structure provided does not define a variable within the expression.
	pub fn eval<'expression>(
		&'expression self,
		variables: &'expression impl Variables,
	) -> Result<u32, Error<'expression>> {
		self.root.eval(self, variables)
	}

	fn parse_pairs(pairs: Pairs<Rule>) -> Self {
		let mut leaves = Vec::new();
		let mut identifiers = Vec::new();

		let root = PRATT_PARSER
			.map_primary(|primary| match primary.as_rule() {
				Rule::integer => Expression::Integer(primary.as_str().parse().unwrap()),
				Rule::identifier => {
					let start = identifiers.len();
					identifiers.extend(primary.as_str().as_ascii().unwrap());
					let end = identifiers.len();
					Expression::Variable(start, end)
				}
				rule => unreachable!(
					"Expr::parse expected terminal value, found {rule:?} ({})",
					primary.as_str()
				),
			})
			.map_infix(|lhs, op, rhs| {
				leaves.push(lhs);
				let lhs = leaves.len() - 1;
				leaves.push(rhs);
				let rhs = leaves.len() - 1;

				match op.as_rule() {
					Rule::add => Expression::Add(lhs, rhs),
					Rule::sub => Expression::Sub(lhs, rhs),
					Rule::mul => Expression::Mul(lhs, rhs),
					Rule::div => Expression::Div(lhs, rhs),
					rule => unreachable!("Expr::parse expected infix operation, found {rule:?}"),
				}
			})
			.parse(pairs);
		Self {
			root,
			leaves,
			identifiers,
		}
	}
}

#[derive(pest_derive::Parser)]
#[grammar = "expression.pest"]
struct ExpressionParser;

lazy_static::lazy_static! {
	static ref PRATT_PARSER: PrattParser<Rule> = {
		use pest::pratt_parser::{Assoc::*, Op};
		use Rule::*;

		// Precedence is defined lowest to highest
		PrattParser::new()
			// Addition and subtract have equal precedence
			.op(Op::infix(add, Left) | Op::infix(sub, Left))
			.op(Op::infix(mul, Left) | Op::infix(div, Left))
	};
}
