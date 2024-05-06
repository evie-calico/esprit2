use pest::iterators::Pairs;
use pest::pratt_parser::PrattParser;
use pest::Parser;
use rand::Rng;
use std::str::FromStr;

pub type Integer = i64;

#[derive(Clone, Debug)]
enum Expression {
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
	) -> Result<Integer, Error<'expression>> {
		let get_leaf = |i: usize| equation.leaves.get(i).unwrap().eval(equation, variables);

		match self {
			Expression::Integer(i) => Ok(*i),
			Expression::Variable(from, to) => variables.get(&equation.source[*from..*to]),
			Expression::Roll(amount, die) => {
				Ok((0..*amount).fold(0, |a, _| a + rand::thread_rng().gen_range(1..=*die)))
			}
			Expression::Add(a, b) => Ok(get_leaf(*a)? + get_leaf(*b)?),
			Expression::Sub(a, b) => Ok(get_leaf(*a)? - get_leaf(*b)?),
			Expression::Mul(a, b) => Ok(get_leaf(*a)? * get_leaf(*b)?),
			Expression::Div(a, b) => Ok(get_leaf(*a)? / get_leaf(*b)?),
			Expression::AddC(x, i) => Ok(get_leaf(*x)? + i),
			Expression::SubC(x, i) => Ok(get_leaf(*x)? - i),
			Expression::MulC(x, i) => Ok(get_leaf(*x)? * i),
			Expression::DivC(x, i) => Ok(get_leaf(*x)? / i),
		}
	}
}

pub trait Variables {
	/// # Errors
	///
	/// Should return `Err(expression::Error::MissingVariable(s)` if a variable is not defined.
	fn get<'expression>(&self, s: &'expression str) -> Result<Integer, Error<'expression>>;
}

impl Variables for () {
	fn get<'expression>(&self, s: &'expression str) -> Result<Integer, Error<'expression>> {
		Err(Error::NoVariables(s))
	}
}

#[derive(Clone, Debug)]
pub struct Equation {
	source: String,
	root: Expression,
	leaves: Vec<Expression>,
}

impl FromStr for Equation {
	type Err = pest::error::Error<Rule>;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(Equation::parse_pairs(
			s.to_string(),
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
	) -> Result<Integer, Error<'expression>> {
		self.root.eval(self, variables)
	}

	fn parse_pairs(source: String, pairs: Pairs<Rule>) -> Self {
		let mut leaves = Vec::new();

		let mut add_leaf = |leaf: Expression| -> usize {
			leaves.push(leaf);
			leaves.len() - 1
		};

		let root =
			PRATT_PARSER
				.map_primary(|primary| match primary.as_rule() {
					Rule::integer => Expression::Integer(primary.as_str().parse().unwrap()),
					Rule::identifier => {
						let span = primary.as_span();
						Expression::Variable(span.start(), span.end())
					}
					Rule::roll => {
						let (amount, die) = primary.as_str().split_once('d').unwrap();
						Expression::Roll(amount.parse().unwrap(), die.parse().unwrap())
					}
					rule => unreachable!(
						"Expr::parse expected terminal value, found {rule:?} ({})",
						primary.as_str()
					),
				})
				.map_infix(|lhs, op, rhs| match (lhs, op.as_rule(), rhs) {
					// Constant resolution
					(Expression::Integer(i), Rule::add, x)
					| (x, Rule::add, Expression::Integer(i)) => Expression::AddC(add_leaf(x), i),
					(Expression::Integer(i), Rule::sub, x)
					| (x, Rule::sub, Expression::Integer(i)) => Expression::SubC(add_leaf(x), i),
					(Expression::Integer(i), Rule::mul, x)
					| (x, Rule::mul, Expression::Integer(i)) => Expression::MulC(add_leaf(x), i),
					(Expression::Integer(i), Rule::div, x)
					| (x, Rule::div, Expression::Integer(i)) => Expression::DivC(add_leaf(x), i),
					(lhs, Rule::add, rhs) => Expression::Add(add_leaf(lhs), add_leaf(rhs)),
					(lhs, Rule::sub, rhs) => Expression::Sub(add_leaf(lhs), add_leaf(rhs)),
					(lhs, Rule::mul, rhs) => Expression::Mul(add_leaf(lhs), add_leaf(rhs)),
					(lhs, Rule::div, rhs) => Expression::Div(add_leaf(lhs), add_leaf(rhs)),
					rule => unreachable!("Expr::parse expected infix operation, found {rule:?}"),
				})
				.parse(pairs);
		Self {
			source,
			root,
			leaves,
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
