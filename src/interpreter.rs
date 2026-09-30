use std::{collections::HashMap, iter::zip};
use std::fmt;
use std::fmt::Write;

use crate::{lexer::TokenType, parser::{ASTNode, ASTNodeEnum}};

#[derive(thiserror::Error, Debug)]
pub enum EvaluatorError {
	#[error("line {row} at {column}: incompatible operation")]
	IncompatibleOperationType {row: usize, column: usize},
	#[error("line {row} at {column}: not known variable '{name}' in this scope")]
	UnknownInThisScope {row: usize, column: usize, name: String},
	#[error("line {row} at {column}: too few arguments for function {function}")]
	TooFewArgumentsForFunction {row: usize, column: usize, function: String},
	#[error("line {row} at {column}: too many arguments for function {function}")]
	TooManyArgumentsForFunction {row: usize, column: usize, function: String},
	#[error("line {row} at {column}: unknown function {name}")]
	UnknownFunction {row: usize, column: usize, name: String},
	#[error("formatting error: {0}")]
	FormatError (#[from] fmt::Error),
	#[error("Break from for/while")]
	BreakError(EvaluatorResultWrapper),
	// #[error("line {row} at {column}: division by zero")]
	// DivisionByZero {row: usize, column: usize},
}

#[derive(Clone, PartialEq, Debug)]
pub enum EvaluatorTypes {
	None,
	Type,
	Boolean,
	Number,
	String,
	Range,
	Tuple(Vec<EvaluatorTypes>),
	TypedTuple(Box<EvaluatorTypes>),
	Function,
	Everything,
}

impl fmt::Display for EvaluatorTypes {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			EvaluatorTypes::None => write!(f, "none"),
			EvaluatorTypes::Boolean => write!(f, "boolean"),
			EvaluatorTypes::Number => write!(f, "number"),
			EvaluatorTypes::String => write!(f, "string"),
			EvaluatorTypes::Range => write!(f, "range"),
			EvaluatorTypes::Tuple(types) => {
				write!(f, "(")?;

				for (i, value) in types.iter().enumerate() {
					if i > 0 {
						write!(f, ", ")?;
					}

					write!(f, "{}", value)?;
				}

				write!(f, ")")
			},
			EvaluatorTypes::TypedTuple(tuple_type) => {
				write!(f, "typed_tuple({})", tuple_type)
			},
			EvaluatorTypes::Function => write!(f, "function"),
			EvaluatorTypes::Type => write!(f, "type"),
			EvaluatorTypes::Everything => write!(f, "..."),
		}
	}
}

impl EvaluatorTypes {
	fn len(&self) -> Option<usize> {
		match self {
			EvaluatorTypes::None => Some(0),
			EvaluatorTypes::Tuple(types) => {
				let mut result = Some(0);

				for eval_type in types {
					match (result, eval_type.len()) {
						(Some(result_len), Some(eval_type)) if eval_type > result_len => result = Some(eval_type),
						(Some(_), None) => result = None,
						_ => {},
					}
				}

				result
			},
			EvaluatorTypes::TypedTuple(_) | EvaluatorTypes::Everything => None,
			_ => Some(1),
		}
	}
}

#[derive(Clone, Debug)]
pub enum EvaluatorResult {
	None,
	Everything,
	Type(EvaluatorTypes),
	Boolean(bool),
	Number(f64),
	String(String),
	Range(f64, f64, f64),
	Tuple(Vec<EvaluatorResultWrapper>),
	Function {
		name: Option<String>,
		args: EvaluatorTypes,
		static_args: Option<Vec<EvaluatorResultWrapper>>,
		func: fn(&mut Evaluator, row: usize, column: usize, args: EvaluatorResultWrapper, static_args: Option<Vec<EvaluatorResultWrapper>>) -> Result<EvaluatorResultWrapper, EvaluatorError>,
	},
	AST(ASTNode),
}

#[derive(Clone, Debug)]
pub struct EvaluatorResultWrapper {
	pub value: EvaluatorResult,
	pub row: usize, pub column: usize,
}

impl EvaluatorResult {
	pub fn to_str(&self, f: &mut String) -> fmt::Result {
		match self {
			EvaluatorResult::None | EvaluatorResult::AST(_) => write!(f, "()"),
			EvaluatorResult::Everything => write!(f, "everything"),
			EvaluatorResult::Number(x) => write!(f, "{}", x),
			EvaluatorResult::String(text) => write!(f, "{}", text),
			EvaluatorResult::Tuple(values) => {
				write!(f, "(")?;

				for (i, value) in values.iter().enumerate() {
					if i > 0 {
						write!(f, ", ")?;
					}

					value.repr(f)?;
				}

				write!(f, ")")
			},
			_ => self.repr(f)
		}
	}

	fn repr(&self, f: &mut String) -> fmt::Result {
		match self {
			EvaluatorResult::None => write!(f, "()"),
			EvaluatorResult::Everything => write!(f, "..."),
			EvaluatorResult::Number(x) => write!(f, "{}", x),
			EvaluatorResult::Boolean(x) => write!(f, "{}", x),
			EvaluatorResult::String(text) => write!(f, "\"{}\"", text),
			&EvaluatorResult::Range(start, end, step) => {
				if step != 1.0 {
					write!(f, "{}..{}..{}", start, end, step)
				} else {
					write!(f, "{}..{}", start, end)
				}
			},
			EvaluatorResult::Tuple(values) => {
				write!(f, "(")?;

				for (i, value) in values.iter().enumerate() {
					if i > 0 {
						write!(f, ", ")?;
					}

					value.repr(f)?;
				}

				write!(f, ")")
			},
			EvaluatorResult::Function { name: Some(name), args, static_args: _, func: _ } => {
				write!(f, "function {}({})", name, args.to_string())
			}
			EvaluatorResult::Function { name: None, args, static_args: _, func: _ } => {
				write!(f, "anonymous function({})", args.to_string())
			}
			EvaluatorResult::AST(ast) => {
				write!(f, "{}", ast)
			},
			EvaluatorResult::Type(x) => write!(f, "{}", x),
		}
	}

	fn get_type(&self) -> EvaluatorResultWrapper {
		match self {
			EvaluatorResult::None | EvaluatorResult::AST(_) => {
				EvaluatorResultWrapper::from_type(EvaluatorTypes::None)
			},
			EvaluatorResult::Everything => {
				EvaluatorResultWrapper::from_type(EvaluatorTypes::Everything)
			},
			EvaluatorResult::Number(_) => {
				EvaluatorResultWrapper::from_type(EvaluatorTypes::Number)
			},
			EvaluatorResult::String(_) => {
				EvaluatorResultWrapper::from_type(EvaluatorTypes::String)
			},
			EvaluatorResult::Range(_, _, _) => {
				EvaluatorResultWrapper::from_type(EvaluatorTypes::Range)
			},
			EvaluatorResult::Boolean(_) => {
				EvaluatorResultWrapper::from_type(EvaluatorTypes::Boolean)
			},
			EvaluatorResult::Type(_) => {
				EvaluatorResultWrapper::from_type(EvaluatorTypes::Type)
			},
			EvaluatorResult::Tuple(values) => {
				let mut types = Vec::new();

				for value in values {
					types.push(value.value.get_type());
				}

				EvaluatorResultWrapper::tuple(types)
			},
			EvaluatorResult::Function { name: _, args: _, static_args: _, func: _ } => {
				EvaluatorResultWrapper::from_type(EvaluatorTypes::Function)
			},
		}
	}

	// fn type_name_static(&self) -> Option<&'static str> {
	// 	match self {
	// 		EvaluatorResult::None => {
	// 			Some("none")
	// 		},
	// 		EvaluatorResult::Number(_) => {
	// 			Some("number")
	// 		},
	// 		EvaluatorResult::String(_) => {
	// 			Some("string")
	// 		},
	// 		EvaluatorResult::Boolean(_) => {
	// 			Some("boolean")
	// 		},
	// 		_ => None
	// 	}
	// }

	// fn type_name_string(&self) -> Result<String, fmt::Error> {
	// 	match self {
	// 		EvaluatorResult::Tuple(values) => {
	// 			let mut result = String::from("(");

	// 			for (i, wrapper) in values.iter().enumerate() {
	// 				if i > 0 {
	// 					write!(&mut result, ", ")?;
	// 				}

	// 				write!(&mut result, "{}", wrapper.value.type_name_string()?)?;
	// 			}

	// 			write!(&mut result, ")")?;

	// 			Ok(result)
	// 		}
	// 		EvaluatorResult::Function { name: _, args: _, func: _ } => {
	// 			let mut result = String::new();

	// 			self.repr(&mut result)?;

	// 			Ok(result)
	// 		}
	// 		_ if let Some(name) = self.type_name_static() => Ok(String::from(name)),
	// 		_ => todo!()
	// 	}
	// }
}

impl fmt::Display for EvaluatorResult {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let mut result = String::new();

		self.to_str(&mut result)?;

		write!(f, "{}", result)
	}
}

impl fmt::Display for EvaluatorResultWrapper {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}", self.value)
	}
}

impl EvaluatorResultWrapper {
	pub const NONE: Self = Self { value: EvaluatorResult::None, column: 0, row: 0 };

	fn from_result(value: EvaluatorResult) -> Self {
		Self {value: value, row: 0, column: 0}
	}

	fn from_result_with_pos(row: usize, column: usize, value: EvaluatorResult) -> Self {
		Self {value: value, row: row, column: column}
	}

	fn to_str(&self, f: &mut String) -> fmt::Result {
		self.value.to_str(f)
	}

	fn to_tuple(self, row: usize, column: usize) -> Result<Self, EvaluatorError> {
		Ok(match self {
			EvaluatorResultWrapper { value: EvaluatorResult::None, column: _, row: _ }
				=> EvaluatorResultWrapper::tuple_with_pos(row, column, Vec::new()),
			EvaluatorResultWrapper { value: EvaluatorResult::Range(start, end, step), column: _, row: _ } => {
				let mut result = Vec::new();

				let mut i = start;

				while i <= end {
					result.push(EvaluatorResultWrapper::number(i));

					i += step;
				}

				EvaluatorResultWrapper::tuple_with_pos(row, column, result)
			},
			EvaluatorResultWrapper { value: EvaluatorResult::Tuple(_), column: _, row: _ } => {
				self.clone()
			},
			_ => {
				EvaluatorResultWrapper::tuple_with_pos(row, column, vec![self])
			},// Err(EvaluatorError::IncompatibleOperationType { row: row, column: column })?,
		})
	}

	fn repr(&self, f: &mut String) -> fmt::Result {
		self.value.repr(f)
	}

	// fn none() -> Self {
	// 	Self::from_result(EvaluatorResult::None)
	// }
	// fn everything() -> Self {
	// 	Self::from_result(EvaluatorResult::Everything)
	// }
	fn from_type(value: EvaluatorTypes) -> Self {
		Self::from_result(EvaluatorResult::Type(value))
	}
	fn boolean(value: bool) -> Self {
		Self::from_result(EvaluatorResult::Boolean(value))
	}
	fn number(value: f64) -> Self {
		Self::from_result(EvaluatorResult::Number(value))
	}
	fn string(value: String) -> Self {
		Self::from_result(EvaluatorResult::String(value))
	}
	fn tuple(value: Vec<EvaluatorResultWrapper>) -> Self {
		Self::from_result(EvaluatorResult::Tuple(value))
	}
	fn function(name: Option<String>, args: EvaluatorTypes, static_args: Option<Vec<EvaluatorResultWrapper>>, func: fn(&mut Evaluator, usize, usize, EvaluatorResultWrapper, Option<Vec<EvaluatorResultWrapper>>) -> Result<EvaluatorResultWrapper, EvaluatorError>) -> Self {
		Self::from_result(EvaluatorResult::Function { name, args, static_args, func })
	}
	fn ast(ast: ASTNode) -> Self {
		Self::from_result(EvaluatorResult::AST(ast))
	}

	// fn none() -> Self {
	// 	Self::from_result(EvaluatorResult::None)
	// }
	// fn everything() -> Self {
	// 	Self::from_result(EvaluatorResult::Everything)
	// }
	// fn from_type_with_pos(row: usize, column: usize, value: EvaluatorTypes) -> Self {
	// 	Self::from_result_with_pos(row, column, EvaluatorResult::Type(value))
	// }
	fn boolean_with_pos(row: usize, column: usize, value: bool) -> Self {
		Self::from_result_with_pos(row, column, EvaluatorResult::Boolean(value))
	}
	fn number_with_pos(row: usize, column: usize, value: f64) -> Self {
		Self::from_result_with_pos(row, column, EvaluatorResult::Number(value))
	}
	// fn string(value: String) -> Self {
	// 	Self::from_result(EvaluatorResult::String(value))
	// }
	fn tuple_with_pos(row: usize, column: usize, value: Vec<EvaluatorResultWrapper>) -> Self {
		Self::from_result_with_pos(row, column, EvaluatorResult::Tuple(value))
	}
	// fn function_with_pos(row: usize, column: usize, name: String, args: EvaluatorTypes, func: fn(usize, usize, EvaluatorResultWrapper) -> Result<EvaluatorResultWrapper, EvaluatorError>) -> Self {
	// 	Self::from_result_with_pos(row, column, EvaluatorResult::Function { name, args, func })
	// }

	fn add(self, rhs: Self) -> Result<Self, EvaluatorError> {
		let result = match (self.value, rhs.value) {
			(EvaluatorResult::String(text_a), EvaluatorResult::String(text_b))
				=> EvaluatorResult::String(format!("{}{}", text_a, text_b)),
			(EvaluatorResult::Number(a), EvaluatorResult::Number(b))
				=> EvaluatorResult::Number(a + b),

			(EvaluatorResult::Tuple(values_a), EvaluatorResult::Tuple(values_b)) => {
				let mut result = Vec::new();

				for (a, b) in zip(values_a, values_b) {
					result.push(a.add(b)?);
				}

				EvaluatorResult::Tuple(result)
			},
			(EvaluatorResult::Tuple(values), EvaluatorResult::Number(b)) |
			(EvaluatorResult::Number(b), EvaluatorResult::Tuple(values)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(value.add(
						EvaluatorResultWrapper::number(b)
					)?);
				}

				EvaluatorResult::Tuple(result)
			},
			_ => return Err(EvaluatorError::IncompatibleOperationType { row: self.row, column: self.column }),
		};

		Ok(EvaluatorResultWrapper {value: result, column: self.column, row: self.row})
	}

	fn sub(self, rhs: Self) -> Result<Self, EvaluatorError> {
		let (row, column) = (self.row, self.column);

		let result = match (self, rhs) {
			(EvaluatorResultWrapper { value: EvaluatorResult::Number(a), row: _, column: _ },
			EvaluatorResultWrapper { value: EvaluatorResult::Number(b), row: _, column: _ })
				=> EvaluatorResult::Number(a - b),

			(EvaluatorResultWrapper { value: EvaluatorResult::Tuple(values_a), row: _, column: _ },
			EvaluatorResultWrapper { value: EvaluatorResult::Tuple(values_b), row: _, column: _ }) => {
				let mut result = Vec::new();

				for (a, b) in zip(values_a, values_b) {
					result.push(a.sub(b)?);
				}

				EvaluatorResult::Tuple(result)
			},

			(EvaluatorResultWrapper { value: EvaluatorResult::Tuple(values), row: _, column: _ },
			EvaluatorResultWrapper { value: EvaluatorResult::Number(b), row: _, column: _ }) => {
				let mut result = Vec::new();

				for value in values {
					result.push(value.sub(
						EvaluatorResultWrapper::number(b)
					)?);
				}

				EvaluatorResult::Tuple(result)
			},
			(b @ EvaluatorResultWrapper { value: EvaluatorResult::Number(_), row: _, column: _ },
			EvaluatorResultWrapper { value: EvaluatorResult::Tuple(values), row: _, column: _ }) => {
				let mut result = Vec::new();

				for value in values {
					result.push(b.clone().sub(value)?);
				}

				EvaluatorResult::Tuple(result)
			},
			_ => return Err(EvaluatorError::IncompatibleOperationType { row: row, column: column }),
		};

		Ok(EvaluatorResultWrapper {value: result, column: column, row: row})
	}

	fn mul(self, rhs: Self) -> Result<Self, EvaluatorError> {
		let result = match (self.value, rhs.value) {
			(EvaluatorResult::Number(a), EvaluatorResult::Number(b))
				=> EvaluatorResult::Number(a * b),
			(EvaluatorResult::Tuple(values), EvaluatorResult::Number(b)) |
			(EvaluatorResult::Number(b), EvaluatorResult::Tuple(values)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(value.mul(
						EvaluatorResultWrapper::number(b)
					)?);
				}

				EvaluatorResult::Tuple(result)
			},

			(EvaluatorResult::Tuple(values_a), EvaluatorResult::Tuple(values_b)) => {
				let mut result = Vec::new();

				for (a, b) in zip(values_a, values_b) {
					result.push(a.mul(b)?);
				}

				EvaluatorResult::Tuple(result)
			},
			_ => return Err(EvaluatorError::IncompatibleOperationType { row: self.row, column: self.column }),
		};

		Ok(EvaluatorResultWrapper {value: result, column: self.column, row: self.row})
	}

	fn div(self, rhs: Self) -> Result<Self, EvaluatorError> {
		let (row, column) = (self.row, self.column);

		let result = match (self, rhs) {
			(EvaluatorResultWrapper { value: EvaluatorResult::Number(a), row: _, column: _ },
				EvaluatorResultWrapper { value: EvaluatorResult::Number(b), row: _, column: _ })
				=> EvaluatorResult::Number(a / b),

			(EvaluatorResultWrapper { value: EvaluatorResult::Tuple(values_a), row: _, column: _ },
			EvaluatorResultWrapper { value: EvaluatorResult::Tuple(values_b), row: _, column: _ }) => {
				let mut result = Vec::new();

				for (a, b) in zip(values_a, values_b) {
					result.push(a.div(b)?);
				}

				EvaluatorResult::Tuple(result)
			},
			(EvaluatorResultWrapper { value: EvaluatorResult::Tuple(values), row: _, column: _ },
				EvaluatorResultWrapper { value: EvaluatorResult::Number(b), row: _, column: _ }) => {
				let mut result = Vec::new();

				for value in values {
					result.push(value.div(
						EvaluatorResultWrapper::number(b)
					)?);
				}

				EvaluatorResult::Tuple(result)
			},
			(b @ EvaluatorResultWrapper { value: EvaluatorResult::Number(_), row: _, column: _ },
			EvaluatorResultWrapper { value: EvaluatorResult::Tuple(values), row: _, column: _ }) => {
				let mut result = Vec::new();

				for value in values {
					result.push(b.clone().div(value)?);
				}

				EvaluatorResult::Tuple(result)
			},
			_ => return Err(EvaluatorError::IncompatibleOperationType { row: row, column: column }),
		};

		Ok(EvaluatorResultWrapper {value: result, column: column, row: row})
	}

	fn remainder(self, rhs: Self) -> Result<Self, EvaluatorError> {
		let (row, column) = (self.row, self.column);

		let result = match (self, rhs) {
			(EvaluatorResultWrapper { value: EvaluatorResult::Number(a), row: _, column: _ },
				EvaluatorResultWrapper { value: EvaluatorResult::Number(b), row: _, column: _ })
				=> EvaluatorResult::Number(a % b),

			(EvaluatorResultWrapper { value: EvaluatorResult::Tuple(values_a), row: _, column: _ },
			EvaluatorResultWrapper { value: EvaluatorResult::Tuple(values_b), row: _, column: _ }) => {
				let mut result = Vec::new();

				for (a, b) in zip(values_a, values_b) {
					result.push(a.remainder(b)?);
				}

				EvaluatorResult::Tuple(result)
			},
			(EvaluatorResultWrapper { value: EvaluatorResult::Tuple(values), row: _, column: _ },
				EvaluatorResultWrapper { value: EvaluatorResult::Number(b), row: _, column: _ }) => {
				let mut result = Vec::new();

				for value in values {
					result.push(value.remainder(
						EvaluatorResultWrapper::number(b)
					)?);
				}

				EvaluatorResult::Tuple(result)
			},
			(b @ EvaluatorResultWrapper { value: EvaluatorResult::Number(_), row: _, column: _ },
			EvaluatorResultWrapper { value: EvaluatorResult::Tuple(values), row: _, column: _ }) => {
				let mut result = Vec::new();

				for value in values {
					result.push(b.clone().remainder(value)?);
				}

				EvaluatorResult::Tuple(result)
			},
			_ => return Err(EvaluatorError::IncompatibleOperationType { row: row, column: column }),
		};

		Ok(EvaluatorResultWrapper {value: result, column: column, row: row})
	}

	fn equals(self, rhs: Self) -> Result<Self, EvaluatorError> {
		let (row, column) = (self.row, self.column);

		let result = match (self, rhs) {
			(EvaluatorResultWrapper { value: EvaluatorResult::Number(a), row: _, column: _ },
			EvaluatorResultWrapper { value: EvaluatorResult::Number(b), row: _, column: _ })
				=> EvaluatorResult::Boolean(a == b),
			(EvaluatorResultWrapper { value: EvaluatorResult::String(a), row: _, column: _ },
			EvaluatorResultWrapper { value: EvaluatorResult::String(b), row: _, column: _ })
				=> EvaluatorResult::Boolean(a == b),
			(EvaluatorResultWrapper { value: EvaluatorResult::Boolean(a), row: _, column: _ },
			EvaluatorResultWrapper { value: EvaluatorResult::Boolean(b), row: _, column: _ })
				=> EvaluatorResult::Boolean(a == b),
			(EvaluatorResultWrapper { value: EvaluatorResult::Type(a), row: _, column: _ },
			EvaluatorResultWrapper { value: EvaluatorResult::Type(b), row: _, column: _ })
				=> EvaluatorResult::Boolean(a == b),

			(EvaluatorResultWrapper { value: EvaluatorResult::Tuple(values), row: _, column: _ },
			b @ EvaluatorResultWrapper { value: EvaluatorResult::Number(_), row: _, column: _ }) |

	 		(b @ EvaluatorResultWrapper { value: EvaluatorResult::Number(_), row: _, column: _ },
			EvaluatorResultWrapper { value: EvaluatorResult::Tuple(values), row: _, column: _ }) => {
				let mut result = Vec::new();

				for value in values {
					result.push(value.equals(b.clone())?);
				}

				EvaluatorResult::Tuple(result)
			},

			(EvaluatorResultWrapper { value: EvaluatorResult::Tuple(values_a), row: _, column: _ },
			EvaluatorResultWrapper { value: EvaluatorResult::Tuple(values_b), row: _, column: _ }) => {
				let mut result = Vec::new();

				for (value_a, value_b) in zip(values_a, values_b) {
					result.push(value_a.equals(value_b)?);
				}

				EvaluatorResult::Tuple(result)
			},
			_ => return Err(EvaluatorError::IncompatibleOperationType { row: row, column: column }),
		};

		Ok(EvaluatorResultWrapper {value: result, column: column, row: row})
	}

	fn not_equals(self, rhs: Self) -> Result<Self, EvaluatorError> {
		let result = match (self.value, rhs.value) {
			(EvaluatorResult::Number(a), EvaluatorResult::Number(b))
				=> EvaluatorResult::Boolean(a != b),
			(EvaluatorResult::String(a), EvaluatorResult::String(b))
				=> EvaluatorResult::Boolean(a != b),
			(EvaluatorResult::Boolean(a), EvaluatorResult::Boolean(b))
				=> EvaluatorResult::Boolean(a != b),
			(EvaluatorResult::Type(a), EvaluatorResult::Type(b))
				=> EvaluatorResult::Boolean(a != b),
			(EvaluatorResult::Tuple(values_a), EvaluatorResult::Tuple(values_b)) => {
				let mut result = Vec::new();

				for (value_a, value_b) in zip(values_a, values_b) {
					result.push(value_a.not_equals(value_b)?);
				}

				EvaluatorResult::Tuple(result)
			},
			_ => return Err(EvaluatorError::IncompatibleOperationType { row: self.row, column: self.column }),
		};

		Ok(EvaluatorResultWrapper {value: result, column: self.column, row: self.row})
	}

	fn all(self) -> Result<bool, EvaluatorError> {
		match self.value {
			EvaluatorResult::Boolean(value) => Ok(value),
			EvaluatorResult::Tuple(values) => {
				let mut result = true;

				for value in values {
					if !value.all()? {
						result = false;
						break;
					}
				}

				Ok(result)
			},
			_ => Err(EvaluatorError::IncompatibleOperationType { row: self.row, column: self.column }),
		}
	}

	fn any(self) -> Result<bool, EvaluatorError> {
		match self.value {
			EvaluatorResult::Boolean(value) => Ok(value),
			EvaluatorResult::Tuple(values) => {
				let mut result = false;

				for value in values {
					if value.any()? {
						result = true;
						break;
					}
				}

				Ok(result)
			},
			_ => Err(EvaluatorError::IncompatibleOperationType { row: self.row, column: self.column }),
		}
	}

	fn map(self, evaluator: &mut Evaluator, name: Option<&str>, args_value: EvaluatorTypes, static_args: Option<Vec<EvaluatorResultWrapper>>, func: fn(&mut Evaluator, usize, usize, EvaluatorResultWrapper, Option<Vec<EvaluatorResultWrapper>>) -> Result<EvaluatorResultWrapper, EvaluatorError>) -> Result<EvaluatorResultWrapper, EvaluatorError> {
		let (row, column) = (self.row, self.column);

		let result = match self {
			EvaluatorResultWrapper { value: EvaluatorResult::Tuple(values), row: _, column: _ } => {
				let mut result = Vec::new();

				for value in values {
					result.push(value.map(evaluator, name, args_value.clone(), static_args.clone(), func)?);
				}

				EvaluatorResultWrapper::tuple_with_pos(row, column, result)
			},
			EvaluatorResultWrapper { value: EvaluatorResult::Range(start, end, step), row: _, column: _ } => {
				let mut result = Vec::new();

				let mut i = start;

				while i <= end {
					result.push(EvaluatorResultWrapper::number(i).map(evaluator, name, args_value.clone(), static_args.clone(), func)?);

					i += step;
				};

				EvaluatorResultWrapper::tuple_with_pos(row, column, result)
			},
			value => evaluator.call_func(name, func, row, column, value, args_value, static_args)?,
		};

		Ok(result)
	}

	fn great_or_equals(self, rhs: Self) -> Result<Self, EvaluatorError> {
		let result = match (self.value, rhs.value) {
			(EvaluatorResult::Number(a), EvaluatorResult::Number(b))
				=> EvaluatorResult::Boolean(a >= b),
			(EvaluatorResult::String(a), EvaluatorResult::String(b))
				=> EvaluatorResult::Boolean(a >= b),
			(EvaluatorResult::Tuple(values_a), EvaluatorResult::Tuple(values_b)) => {
				let mut result = Vec::new();

				for (value_a, value_b) in zip(values_a, values_b) {
					result.push(value_a.great_or_equals(value_b)?);
				}

				EvaluatorResult::Tuple(result)
			},
			_ => return Err(EvaluatorError::IncompatibleOperationType { row: self.row, column: self.column }),
		};

		Ok(EvaluatorResultWrapper {value: result, column: self.column, row: self.row})
	}

	fn less_or_equals(self, rhs: Self) -> Result<Self, EvaluatorError> {
		let result = match (self.value, rhs.value) {
			(EvaluatorResult::Number(a), EvaluatorResult::Number(b))
				=> EvaluatorResult::Boolean(a <= b),
			(EvaluatorResult::String(a), EvaluatorResult::String(b))
				=> EvaluatorResult::Boolean(a <= b),
			(EvaluatorResult::Tuple(values_a), EvaluatorResult::Tuple(values_b)) => {
				let mut result = Vec::new();

				for (value_a, value_b) in zip(values_a, values_b) {
					result.push(value_a.less_or_equals(value_b)?);
				}

				EvaluatorResult::Tuple(result)
			},
			_ => return Err(EvaluatorError::IncompatibleOperationType { row: self.row, column: self.column }),
		};

		Ok(EvaluatorResultWrapper {value: result, column: self.column, row: self.row})
	}

	fn great(self, rhs: Self) -> Result<Self, EvaluatorError> {
		let result = match (self.value, rhs.value) {
			(EvaluatorResult::Number(a), EvaluatorResult::Number(b))
				=> EvaluatorResult::Boolean(a > b),
			(EvaluatorResult::String(a), EvaluatorResult::String(b))
				=> EvaluatorResult::Boolean(a > b),
			(EvaluatorResult::Tuple(values_a), EvaluatorResult::Tuple(values_b)) => {
				let mut result = Vec::new();

				for (value_a, value_b) in zip(values_a, values_b) {
					result.push(value_a.great(value_b)?);
				}

				EvaluatorResult::Tuple(result)
			},
			_ => return Err(EvaluatorError::IncompatibleOperationType { row: self.row, column: self.column }),
		};

		Ok(EvaluatorResultWrapper {value: result, column: self.column, row: self.row})
	}

	fn less(self, rhs: Self) -> Result<Self, EvaluatorError> {
		let result = match (self.value, rhs.value) {
			(EvaluatorResult::Number(a), EvaluatorResult::Number(b))
				=> EvaluatorResult::Boolean(a < b),
			(EvaluatorResult::String(a), EvaluatorResult::String(b))
				=> EvaluatorResult::Boolean(a < b),
			(EvaluatorResult::Tuple(values_a), EvaluatorResult::Tuple(values_b)) => {
				let mut result = Vec::new();

				for (value_a, value_b) in zip(values_a, values_b) {
					result.push(value_a.less(value_b)?);
				}

				EvaluatorResult::Tuple(result)
			},
			_ => return Err(EvaluatorError::IncompatibleOperationType { row: self.row, column: self.column }),
		};

		Ok(EvaluatorResultWrapper {value: result, column: self.column, row: self.row})
	}

	fn logical_and(self, rhs: Self) -> Result<Self, EvaluatorError> {
		let result = match (self.value, rhs.value) {
			(EvaluatorResult::Boolean(a), EvaluatorResult::Boolean(b)) =>
				EvaluatorResult::Boolean(a && b),
			(EvaluatorResult::Tuple(values_a), EvaluatorResult::Tuple(values_b)) => {
				let mut result = Vec::new();

				for (value_a, value_b) in zip(values_a, values_b) {
					result.push(value_a.logical_and(value_b)?);
				}

				EvaluatorResult::Tuple(result)
			},
			_ => return Err(EvaluatorError::IncompatibleOperationType { row: self.row, column: self.column }),
		};

		Ok(EvaluatorResultWrapper {value: result, column: self.column, row: self.row})
	}

	fn logical_or(self, rhs: Self) -> Result<Self, EvaluatorError> {
		let result = match (self.value, rhs.value) {
			(EvaluatorResult::Boolean(a), EvaluatorResult::Boolean(b)) =>
				EvaluatorResult::Boolean(a || b),
			(EvaluatorResult::Tuple(values_a), EvaluatorResult::Tuple(values_b)) => {
				let mut result = Vec::new();

				for (value_a, value_b) in zip(values_a, values_b) {
					result.push(value_a.less(value_b)?);
				}

				EvaluatorResult::Tuple(result)
			},
			_ => return Err(EvaluatorError::IncompatibleOperationType { row: self.row, column: self.column }),
		};

		Ok(EvaluatorResultWrapper {value: result, column: self.column, row: self.row})
	}

	// fn not(self) -> Result<Self, EvaluatorError> {
	// 	let result = match self.value {
	// 		EvaluatorResult::Boolean(x) => EvaluatorResult::Boolean(!x),
	// 		EvaluatorResult::Tuple(values) => {
	// 			let mut result = Vec::new();

	// 			for value in values {
	// 				result.push(value.not()?);
	// 			}

	// 			EvaluatorResult::Tuple(result)
	// 		},
	// 		_ => return Err(EvaluatorError::IncompatibleOperationType { row: self.row, column: self.column }),
	// 	};

	// 	Ok(EvaluatorResultWrapper {value: result, column: self.column, row: self.row})
	// }

	fn neg(self) -> Result<Self, EvaluatorError> {
		let result = match self.value {
			EvaluatorResult::Number(x) => EvaluatorResult::Number(-x),
			EvaluatorResult::Boolean(x) => EvaluatorResult::Boolean(!x),
			EvaluatorResult::Tuple(values) => {
				let mut result = Vec::new();

				for value in values {
					result.push(value.neg()?);
				}

				EvaluatorResult::Tuple(result)
			},
			_ => return Err(EvaluatorError::IncompatibleOperationType { row: self.row, column: self.column }),
		};

		Ok(EvaluatorResultWrapper {value: result, column: self.column, row: self.row})
	}

	fn len(&self) -> usize {
		match self.value {
			EvaluatorResult::None => 0,
			EvaluatorResult::Number(_) => 1,
			EvaluatorResult::Boolean(_) => 1,
			EvaluatorResult::String(ref text) => text.len(),
			EvaluatorResult::Range(start, end, step) => ((end + 1.0 - start).abs() / step) as usize,
			EvaluatorResult::Tuple(ref values) => values.len(),
			_ => 1,
		}
	}

	fn key(&self) -> usize {
		match self.value {
			EvaluatorResult::None => 0,
			EvaluatorResult::Number(x) => x as usize,
			EvaluatorResult::Boolean(x) => if x {1} else {0},
			EvaluatorResult::String(ref text) => text.len(),
			EvaluatorResult::Range(start, end, step) => ((end + 1.0 - start).abs() / step) as usize,
			EvaluatorResult::Tuple(ref values) => values.len(),
			_ => 1,
		}
	}

	fn abs(self) -> Result<EvaluatorResultWrapper, EvaluatorError> {
		let result = match self.value {
			EvaluatorResult::Number(x) =>
				EvaluatorResult::Number(x.abs()),
			EvaluatorResult::Range(start, end, step) => {
				let mut result = Vec::new();

				let mut i = start;

				while i <= end {
					result.push(EvaluatorResultWrapper::number(i).abs()?);

					i += step;
				};

				EvaluatorResult::Tuple(result)
			},
			EvaluatorResult::Tuple(values) => {
				let mut result = Vec::new();

				for value in values {
					result.push(value.abs()?);
				}

				EvaluatorResult::Tuple(result)
			},
			_ => return Err(EvaluatorError::IncompatibleOperationType { row: self.row, column: self.column }),
		};

		Ok(EvaluatorResultWrapper {value: result, column: self.column, row: self.row})
	}

	fn sum(self) -> Result<f64, EvaluatorError> {
		let result = match self.value {
			EvaluatorResult::Number(x) => x,
			EvaluatorResult::Range(start, end, step) => {
				((start + end - 1.0) * (end - start)) / (2.0 * step)
			},
			EvaluatorResult::String(value) => {
				value.len() as f64
			},
			EvaluatorResult::Tuple(values) => {
				let mut result = 0.0;

				for value in values {
					result += value.sum()?;
				}

				result
			},
			_ => return Err(EvaluatorError::IncompatibleOperationType { row: self.row, column: self.column }),
		};

		Ok(result)
	}

	fn min(self) -> Result<EvaluatorResultWrapper, EvaluatorError> {
		let result = match self.value {
			EvaluatorResult::Range(start, end, _) => {
				start.min(end)
			},
			EvaluatorResult::Tuple(values) => {
				values.iter().map(|v| v.key()).min().unwrap_or(0) as f64
			},
			_ => return Err(EvaluatorError::IncompatibleOperationType { row: self.row, column: self.column }),
		};

		Ok(EvaluatorResultWrapper::number_with_pos(self.row, self.column, result))
	}

	fn max(self) -> Result<EvaluatorResultWrapper, EvaluatorError> {
		let result = match self.value {
			EvaluatorResult::Range(start, end, _) => {
				start.max(end)
			},
			EvaluatorResult::Tuple(values) => {
				values.iter().map(|v| v.key()).max().unwrap_or(0) as f64
			},
			_ => return Err(EvaluatorError::IncompatibleOperationType { row: self.row, column: self.column }),
		};

		Ok(EvaluatorResultWrapper::number_with_pos(self.row, self.column, result))
	}
	fn mean(self) -> Result<EvaluatorResultWrapper, EvaluatorError> {
		let len = self.len();

		Ok(EvaluatorResultWrapper::number_with_pos(self.row, self.column, self.sum()? / (len as f64)))
	}

	fn sort(self) -> Result<EvaluatorResultWrapper, EvaluatorError> {
		match self.value {
			EvaluatorResult::Tuple(mut values) => {
				values.sort_by_key(|value| value.key());

				Ok(EvaluatorResultWrapper::tuple_with_pos(self.row, self.column, values))
			},
			_ => Err(EvaluatorError::IncompatibleOperationType { row: self.row, column: self.column }),
		}
	}

	fn median(self) -> Result<EvaluatorResultWrapper, EvaluatorError> {
		let (row, column) = (self.row, self.column);

		let result = self.sort()?;
		let result_len = result.len();

		match result.value {
			EvaluatorResult::Tuple(values) if result_len % 2 != 0 => {
				Ok(values[result_len / 2].clone())
			},
			EvaluatorResult::Tuple(values) if result_len % 2 == 0 => {
				values[result_len / 2 - 1].clone().add(values[result_len / 2].clone())?.div(EvaluatorResultWrapper::number(2.0))
			},
			_ => Err(EvaluatorError::IncompatibleOperationType { row: row, column: column }),
		}
	}

	fn pow(self, right: Self) -> Result<Self, EvaluatorError> {
		let result = match (self.value, right.value) {
			(EvaluatorResult::Number(a), EvaluatorResult::Number(b))
				=> EvaluatorResult::Number(f64::powf(a, b)),
			(EvaluatorResult::Tuple(values), EvaluatorResult::Number(b)) |
			(EvaluatorResult::Number(b), EvaluatorResult::Tuple(values)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(value.pow(
						EvaluatorResultWrapper::number(b)
					)?);
				}

				EvaluatorResult::Tuple(result)
			},
			_ => return Err(EvaluatorError::IncompatibleOperationType { row: self.row, column: self.column }),
		};

		Ok(EvaluatorResultWrapper {value: result, column: self.column, row: self.row})
	}

	fn sqrt(self) -> Result<Self, EvaluatorError> {
		let result = match self.value {
			EvaluatorResult::Number(x)
				=> EvaluatorResult::Number(f64::sqrt(x)),
			EvaluatorResult::Tuple(values) => {
				let mut result = Vec::new();

				for value in values {
					result.push(value.sqrt()?);
				}

				EvaluatorResult::Tuple(result)
			},
			_ => return Err(EvaluatorError::IncompatibleOperationType { row: self.row, column: self.column }),
		};

		Ok(EvaluatorResultWrapper {value: result, column: self.column, row: self.row})
	}

	fn sin(self) -> Result<Self, EvaluatorError> {
		let result = match self.value {
			EvaluatorResult::Number(x)
				=> EvaluatorResult::Number(f64::sin(x)),
			EvaluatorResult::Tuple(values) => {
				let mut result = Vec::new();

				for value in values {
					result.push(value.sin()?);
				}

				EvaluatorResult::Tuple(result)
			},
			_ => return Err(EvaluatorError::IncompatibleOperationType { row: self.row, column: self.column }),
		};

		Ok(EvaluatorResultWrapper {value: result, column: self.column, row: self.row})
	}

	fn cos(self) -> Result<Self, EvaluatorError> {
		let result = match self.value {
			EvaluatorResult::Number(x)
				=> EvaluatorResult::Number(f64::cos(x)),
			EvaluatorResult::Tuple(values) => {
				let mut result = Vec::new();

				for value in values {
					result.push(value.cos()?);
				}

				EvaluatorResult::Tuple(result)
			},
			_ => return Err(EvaluatorError::IncompatibleOperationType { row: self.row, column: self.column }),
		};

		Ok(EvaluatorResultWrapper {value: result, column: self.column, row: self.row})
	}
}

pub struct Evaluator {
	scope: HashMap<String, EvaluatorResultWrapper>,
}

impl Evaluator {
	pub fn new() -> Self {
		Self {scope: HashMap::new()}
	}

	pub fn init_builtin_funcs(&mut self) {
		self.scope.insert(String::from("sqrt"), EvaluatorResultWrapper::function(Some(String::from("sqrt")), EvaluatorTypes::TypedTuple(Box::new(EvaluatorTypes::Number)), None, Self::sqrt));
		self.scope.insert(String::from("sin"), EvaluatorResultWrapper::function(Some(String::from("sin")), EvaluatorTypes::TypedTuple(Box::new(EvaluatorTypes::Number)), None, Self::sin));
		self.scope.insert(String::from("cos"), EvaluatorResultWrapper::function(Some(String::from("cos")), EvaluatorTypes::TypedTuple(Box::new(EvaluatorTypes::Number)), None, Self::cos));
		self.scope.insert(String::from("print"), EvaluatorResultWrapper::function(Some(String::from("print")), EvaluatorTypes::Everything, None, Self::print));
		self.scope.insert(String::from("type"), EvaluatorResultWrapper::function(Some(String::from("type")), EvaluatorTypes::Everything, None, Self::type_name));
		self.scope.insert(String::from("repr"), EvaluatorResultWrapper::function(Some(String::from("repr")), EvaluatorTypes::Everything, None, Self::repr));
		self.scope.insert(String::from("str"), EvaluatorResultWrapper::function(Some(String::from("str")), EvaluatorTypes::Everything, None, Self::to_str));
		self.scope.insert(String::from("tuple"), EvaluatorResultWrapper::function(Some(String::from("tuple")), EvaluatorTypes::Everything, None, Self::to_tuple));
		self.scope.insert(String::from("all"), EvaluatorResultWrapper::function(Some(String::from("all")), EvaluatorTypes::TypedTuple(Box::new(EvaluatorTypes::Boolean)), None, Self::calc_all));
		self.scope.insert(String::from("any"), EvaluatorResultWrapper::function(Some(String::from("any")), EvaluatorTypes::TypedTuple(Box::new(EvaluatorTypes::Boolean)), None, Self::calc_any));
		self.scope.insert(String::from("map"), EvaluatorResultWrapper::function(Some(String::from("map")), EvaluatorTypes::Tuple(vec![EvaluatorTypes::Function, EvaluatorTypes::Everything]), None, Self::calc_map));
		self.scope.insert(String::from("len"), EvaluatorResultWrapper::function(Some(String::from("len")), EvaluatorTypes::Everything, None, Self::calc_len));
		self.scope.insert(String::from("abs"), EvaluatorResultWrapper::function(Some(String::from("abs")), EvaluatorTypes::TypedTuple(Box::new(EvaluatorTypes::Number)), None, Self::calc_abs));
		self.scope.insert(String::from("sum"), EvaluatorResultWrapper::function(Some(String::from("sum")), EvaluatorTypes::Everything, None, Self::calc_sum));
		self.scope.insert(String::from("min"), EvaluatorResultWrapper::function(Some(String::from("min")), EvaluatorTypes::Everything, None, Self::calc_min));
		self.scope.insert(String::from("max"), EvaluatorResultWrapper::function(Some(String::from("max")), EvaluatorTypes::Everything, None, Self::calc_max));
		self.scope.insert(String::from("mean"), EvaluatorResultWrapper::function(Some(String::from("mean")), EvaluatorTypes::Everything, None, Self::calc_mean));
		self.scope.insert(String::from("median"), EvaluatorResultWrapper::function(Some(String::from("median")), EvaluatorTypes::TypedTuple(Box::new(EvaluatorTypes::Number)), None, Self::calc_median));
		self.scope.insert(String::from("sort"), EvaluatorResultWrapper::function(Some(String::from("sort")), EvaluatorTypes::TypedTuple(Box::new(EvaluatorTypes::Everything)), None, Self::calc_sort));
	}

	fn sqrt(&mut self, _row: usize, _column: usize, arg: EvaluatorResultWrapper, _static_args: Option<Vec<EvaluatorResultWrapper>>) -> Result<EvaluatorResultWrapper, EvaluatorError> {
		arg.sqrt()
	}

	fn sin(&mut self, _row: usize, _column: usize, arg: EvaluatorResultWrapper, _static_args: Option<Vec<EvaluatorResultWrapper>>) -> Result<EvaluatorResultWrapper, EvaluatorError> {
		arg.sin()
	}

	fn cos(&mut self, _row: usize, _column: usize, arg: EvaluatorResultWrapper, _static_args: Option<Vec<EvaluatorResultWrapper>>) -> Result<EvaluatorResultWrapper, EvaluatorError> {
		arg.cos()
	}

	fn calc_all(&mut self, _row: usize, _column: usize, arg: EvaluatorResultWrapper, _static_args: Option<Vec<EvaluatorResultWrapper>>) -> Result<EvaluatorResultWrapper, EvaluatorError> {
		Ok(EvaluatorResultWrapper::boolean(arg.all()?))
	}

	fn calc_any(&mut self, row: usize, column: usize, arg: EvaluatorResultWrapper, _static_args: Option<Vec<EvaluatorResultWrapper>>) -> Result<EvaluatorResultWrapper, EvaluatorError> {
		Ok(EvaluatorResultWrapper::boolean_with_pos(row, column, arg.any()?))
	}

	fn calc_len(&mut self, row: usize, column: usize, arg: EvaluatorResultWrapper, _static_args: Option<Vec<EvaluatorResultWrapper>>) -> Result<EvaluatorResultWrapper, EvaluatorError> {
		Ok(EvaluatorResultWrapper::number_with_pos(row, column, arg.len() as f64))
	}

	fn calc_abs(&mut self, _row: usize, _column: usize, arg: EvaluatorResultWrapper, _static_args: Option<Vec<EvaluatorResultWrapper>>) -> Result<EvaluatorResultWrapper, EvaluatorError> {
		arg.abs()
	}
	fn calc_sum(&mut self, row: usize, column: usize, arg: EvaluatorResultWrapper, _static_args: Option<Vec<EvaluatorResultWrapper>>) -> Result<EvaluatorResultWrapper, EvaluatorError> {
		Ok(EvaluatorResultWrapper::number_with_pos(row, column, arg.sum()?))
	}
	fn calc_sort(&mut self, _row: usize, _column: usize, arg: EvaluatorResultWrapper, _static_args: Option<Vec<EvaluatorResultWrapper>>) -> Result<EvaluatorResultWrapper, EvaluatorError> {
		arg.sort()
	}
	fn calc_min(&mut self, _row: usize, _column: usize, arg: EvaluatorResultWrapper, _static_args: Option<Vec<EvaluatorResultWrapper>>) -> Result<EvaluatorResultWrapper, EvaluatorError> {
		arg.min()
	}
	fn calc_max(&mut self, _row: usize, _column: usize, arg: EvaluatorResultWrapper, _static_args: Option<Vec<EvaluatorResultWrapper>>) -> Result<EvaluatorResultWrapper, EvaluatorError> {
		arg.max()
	}
	fn calc_mean(&mut self, _row: usize, _column: usize, arg: EvaluatorResultWrapper, _static_args: Option<Vec<EvaluatorResultWrapper>>) -> Result<EvaluatorResultWrapper, EvaluatorError> {
		arg.mean()
	}
	fn calc_median(&mut self, _row: usize, _column: usize, arg: EvaluatorResultWrapper, _static_args: Option<Vec<EvaluatorResultWrapper>>) -> Result<EvaluatorResultWrapper, EvaluatorError> {
		arg.median()
	}

	fn calc_map(&mut self, row: usize, column: usize, arg: EvaluatorResultWrapper, _static_args: Option<Vec<EvaluatorResultWrapper>>) -> Result<EvaluatorResultWrapper, EvaluatorError> {
		match arg.value {
			EvaluatorResult::Tuple(values) => match values[0].value.clone() {
				EvaluatorResult::Function { name, args, ref static_args, func } => {
					let mut result: Vec<EvaluatorResultWrapper> = Vec::new();

					for value in values.iter().skip(1).cloned() {
						result.push(value.map(self, name.as_deref(), args.clone(), static_args.clone(), func)?);
					}

					Ok(EvaluatorResultWrapper::tuple_with_pos(row, column, result))
				},
				_ => Err(EvaluatorError::IncompatibleOperationType { row, column }),
			},

			_ => Err(EvaluatorError::IncompatibleOperationType { row, column }),
		}
	}

	fn print(&mut self, _row: usize, _column: usize, arg: EvaluatorResultWrapper, _static_args: Option<Vec<EvaluatorResultWrapper>>) -> Result<EvaluatorResultWrapper, EvaluatorError> {
		match arg.value {
			EvaluatorResult::Tuple(values) => {
				let mut result = String::new();

				for value in values {
					value.to_str(&mut result)?;

					write!(&mut result, " ")?;
				}

				println!("{}", result);
			},
			_ => println!("{}", arg),
		}

		Ok(EvaluatorResultWrapper::NONE)
	}

	fn type_name(&mut self, _row: usize, _column: usize, arg: EvaluatorResultWrapper, _static_args: Option<Vec<EvaluatorResultWrapper>>) -> Result<EvaluatorResultWrapper, EvaluatorError> {
		Ok(arg.value.get_type())
	}

	fn repr(&mut self, _row: usize, _column: usize, arg: EvaluatorResultWrapper, _static_args: Option<Vec<EvaluatorResultWrapper>>) -> Result<EvaluatorResultWrapper, EvaluatorError> {
		let mut result = String::new();

		arg.value.repr(&mut result)?;

		Ok(EvaluatorResultWrapper::from_result(EvaluatorResult::String(result)))
	}

	fn to_str(&mut self, _row: usize, _column: usize, arg: EvaluatorResultWrapper, _static_args: Option<Vec<EvaluatorResultWrapper>>) -> Result<EvaluatorResultWrapper, EvaluatorError> {
		let mut result = String::new();

		arg.value.to_str(&mut result)?;

		Ok(EvaluatorResultWrapper::from_result(EvaluatorResult::String(result)))
	}

	fn to_tuple(&mut self, row: usize, column: usize, arg: EvaluatorResultWrapper, _static_args: Option<Vec<EvaluatorResultWrapper>>) -> Result<EvaluatorResultWrapper, EvaluatorError> {
		Ok(arg.to_tuple(row, column)?)
	}

	fn user_func(&mut self, _row: usize, _column: usize, arg: EvaluatorResultWrapper, static_args: Option<Vec<EvaluatorResultWrapper>>) -> Result<EvaluatorResultWrapper, EvaluatorError> {
		let static_args = static_args.expect("static_args.len() < 1");

		match arg.value {
			EvaluatorResult::Tuple(values) => {
				for (name, value) in zip(static_args.iter().skip(1), values.iter()) {
					match name.clone().value {
						EvaluatorResult::String(name) => {
							self.scope.insert(name, value.clone());
						},
						_ => continue,
					}
				}
			}
			_ => {
				let (name, value) = (static_args.iter().nth(1).expect("static_args.len() < 2"), arg);

				match name.clone().value {
					EvaluatorResult::String(name) => {
						self.scope.insert(name, value.clone());
					},
					_ => {},
				}
			},
		}

		let result = match static_args[0].clone().value {
			EvaluatorResult::AST(ast) => self.eval(Box::new(ast))?,
			_ => EvaluatorResultWrapper::NONE
		};

		for name in static_args.iter().skip(1) {
			match name.clone().value {
				EvaluatorResult::String(name) => {
					self.scope.remove(&name);
				},
				_ => continue,
			}
		}

		Ok(result)
	}

	fn call_func(&mut self, name: Option<&str>, func: fn(&mut Evaluator, usize, usize, EvaluatorResultWrapper, Option<Vec<EvaluatorResultWrapper>>) -> Result<EvaluatorResultWrapper, EvaluatorError>, row: usize, column: usize, args: EvaluatorResultWrapper, args_types: EvaluatorTypes, static_args: Option<Vec<EvaluatorResultWrapper>>) -> Result<EvaluatorResultWrapper, EvaluatorError> {
		match (args.len(), args_types.len()) {
			(a, Some(b)) if a < b =>
				return Err(EvaluatorError::TooFewArgumentsForFunction { row: row, column: column, function: name.unwrap_or("<anonymous>").to_string() }),
			(a, Some(b)) if a > b =>
				return Err(EvaluatorError::TooManyArgumentsForFunction { row: row, column: column, function: name.unwrap_or("<anonymous>").to_string() }),

			(_, _) => {},
		}

		func(self, row, column, args, static_args)
	}

	pub fn eval(&mut self, expr: Box<ASTNode>) -> Result<EvaluatorResultWrapper, EvaluatorError> {
		Ok(match expr.value {
			ASTNodeEnum::Everything =>
				EvaluatorResultWrapper { value: EvaluatorResult::Everything, column: expr.column, row: expr.row },
			ASTNodeEnum::Boolean(x) =>
				EvaluatorResultWrapper { value: EvaluatorResult::Boolean(x), column: expr.column, row: expr.row },
			ASTNodeEnum::Number(x) =>
				EvaluatorResultWrapper { value: EvaluatorResult::Number(x), column: expr.column, row: expr.row },
			ASTNodeEnum::String(x) =>
				EvaluatorResultWrapper { value: EvaluatorResult::String(x), column: expr.column, row: expr.row },
			ASTNodeEnum::Variable(name) => {
				match self.scope.get(&name) {
					Some(value) => value.clone(),
					None => return Err(EvaluatorError::UnknownInThisScope { row: expr.row, column: expr.column, name: name })
				}
			},
			ASTNodeEnum::Tuple(values) => {
				let mut result: Vec<EvaluatorResultWrapper> = Vec::new();

				for value in values {
					result.push(self.eval(Box::new(value))?);
				}

				EvaluatorResultWrapper { value: EvaluatorResult::Tuple(result), column: expr.column, row: expr.row }
			},
			ASTNodeEnum::Block(values) => {
				let mut values_iter = values.iter();

				let mut res = match values_iter.next() {
					Some(x) => self.eval(Box::new(x.clone()))?,
					None => return Ok(EvaluatorResultWrapper::NONE)
				};

				for value in values_iter {
					res = self.eval(Box::new(value.clone()))?;
				}

				res
			},
			ASTNodeEnum::If { condition, block, block_else: Some(block_else) } => {
				match self.eval(condition)? {
					EvaluatorResultWrapper { value: EvaluatorResult::Boolean(true), row: _, column: _ } => self.eval(block)?,
					EvaluatorResultWrapper { value: EvaluatorResult::Boolean(false), row: _, column: _ } => self.eval(block_else)?,
					_ => EvaluatorResultWrapper::NONE,
				}
			},
			ASTNodeEnum::If { condition, block, block_else: None } => {
				match self.eval(condition)? {
					EvaluatorResultWrapper { value: EvaluatorResult::Boolean(true), row: _, column: _ } => self.eval(block)?,
					_ => EvaluatorResultWrapper::NONE,
				}
			},

			ASTNodeEnum::Break(value) => {
				return Err(EvaluatorError::BreakError(self.eval(value)?))
			},

			ASTNodeEnum::While { condition, block, block_else: Some(block_else) } => {
				let mut result = EvaluatorResultWrapper::NONE;

				let mut breaked = false;

				loop {
					match self.eval(condition.clone())? {
						EvaluatorResultWrapper { value: EvaluatorResult::Boolean(true), row: _, column: _ } => {},
						_ => break,
					};

					result = match self.eval(block.clone()) {
						Ok(result) => result,
						Err(EvaluatorError::BreakError(value)) => {
							result = value;
							breaked = true;

							break;
						},
						Err(e) => return Err(e),
					};
				}

				if !breaked {
					self.eval(block_else)?;
				}

				result
			},
			ASTNodeEnum::While { condition, block, block_else: None } => {
				let mut result = EvaluatorResultWrapper::NONE;

				loop {
					match self.eval(condition.clone())? {
						EvaluatorResultWrapper { value: EvaluatorResult::Boolean(true), row: _, column: _ } => {},
						_ => break,
					};

					result = match self.eval(block.clone()) {
						Ok(result) => result,
						Err(EvaluatorError::BreakError(value)) => {
							result = value;

							break;
						},
						Err(e) => return Err(e),
					};
				}

				result
			},
			ASTNodeEnum::Binary {left, op: TokenType::TokenAssignment, right}
		 		if let ASTNodeEnum::Variable(name) = left.value.clone() => {
				let result = self.eval(right)?;

				self.scope.insert(name, result.clone());

				result
			},

			ASTNodeEnum::Binary {left, op: TokenType::TokenPlusAssignment, right}
		 		if let ASTNodeEnum::Variable(name) = left.value.clone() => {

				match self.scope.get(&name) {
					Some(value) => {
						let result = value.clone().add(self.eval(right)?)?;

						self.scope.insert(name, result.clone());

						result
					},
					None => return Err(EvaluatorError::UnknownInThisScope { row: expr.row, column: expr.column, name: name })
				}
			},

			ASTNodeEnum::Binary {left, op: TokenType::TokenMinusAssignment, right}
		 		if let ASTNodeEnum::Variable(name) = left.value.clone() => {

				match self.scope.get(&name) {
					Some(value) => {
						let result = value.clone().sub(self.eval(right)?)?;

						self.scope.insert(name, result.clone());

						result
					},
					None => return Err(EvaluatorError::UnknownInThisScope { row: expr.row, column: expr.column, name: name })
				}
			},

			ASTNodeEnum::Binary {left, op: TokenType::TokenPowAssignment, right}
		 		if let ASTNodeEnum::Variable(name) = left.value.clone() => {

				match self.scope.get(&name) {
					Some(value) => {
						let result = value.clone().pow(self.eval(right)?)?;

						self.scope.insert(name, result.clone());

						result
					},
					None => return Err(EvaluatorError::UnknownInThisScope { row: expr.row, column: expr.column, name: name })
				}
			},

			ASTNodeEnum::Binary {left, op: TokenType::TokenMultiplyAssignment, right}
		 		if let ASTNodeEnum::Variable(name) = left.value.clone() => {

				match self.scope.get(&name) {
					Some(value) => {
						let result = value.clone().mul(self.eval(right)?)?;

						self.scope.insert(name, result.clone());

						result
					},
					None => return Err(EvaluatorError::UnknownInThisScope { row: expr.row, column: expr.column, name: name })
				}
			},

			ASTNodeEnum::Binary {left, op: TokenType::TokenDivideAssignment, right}
		 		if let ASTNodeEnum::Variable(name) = left.value.clone() => {

				match self.scope.get(&name) {
					Some(value) => {
						let result = value.clone().div(self.eval(right)?)?;

						self.scope.insert(name, result.clone());

						result
					},
					None => return Err(EvaluatorError::UnknownInThisScope { row: expr.row, column: expr.column, name: name })
				}
			},

			ASTNodeEnum::Binary {left, op: TokenType::TokenRemainderAssignment, right}
		 		if let ASTNodeEnum::Variable(name) = left.value.clone() => {

				match self.scope.get(&name) {
					Some(value) => {
						let result = value.clone().remainder(self.eval(right)?)?;

						self.scope.insert(name, result.clone());

						result
					},
					None => return Err(EvaluatorError::UnknownInThisScope { row: expr.row, column: expr.column, name: name })
				}
			},

			ASTNodeEnum::Binary {left, op: TokenType::TokenPlus, right} =>
				self.eval(left)?.add(self.eval(right)?)?,
			ASTNodeEnum::Binary {left, op: TokenType::TokenMinus, right} =>
				self.eval(left)?.sub(self.eval(right)?)?,
			ASTNodeEnum::Binary {left, op: TokenType::TokenMultiply, right} =>
				self.eval(left)?.mul(self.eval(right)?)?,
			ASTNodeEnum::Binary {left, op: TokenType::TokenPow, right} =>
				self.eval(left)?.pow(self.eval(right)?)?,
			ASTNodeEnum::Binary {left, op: TokenType::TokenDivide, right} =>
				self.eval(left)?.div(self.eval(right)?)?,
			ASTNodeEnum::Binary {left, op: TokenType::TokenRemainder, right} =>
				self.eval(left)?.remainder(self.eval(right)?)?,

			ASTNodeEnum::Binary {left, op: TokenType::TokenEquals, right} =>
				self.eval(left)?.equals(self.eval(right)?)?,
			ASTNodeEnum::Binary {left, op: TokenType::TokenNotEquals, right} =>
				self.eval(left)?.not_equals(self.eval(right)?)?,
			ASTNodeEnum::Binary {left, op: TokenType::TokenGreatOrEquals, right} =>
				self.eval(left)?.great_or_equals(self.eval(right)?)?,
			ASTNodeEnum::Binary {left, op: TokenType::TokenLessOrEquals, right} =>
				self.eval(left)?.less_or_equals(self.eval(right)?)?,
			ASTNodeEnum::Binary {left, op: TokenType::TokenGreat, right} =>
				self.eval(left)?.great(self.eval(right)?)?,
			ASTNodeEnum::Binary {left, op: TokenType::TokenLess, right} =>
				self.eval(left)?.less(self.eval(right)?)?,

			ASTNodeEnum::Binary {left, op: TokenType::TokenLogicalAnd, right} =>
				self.eval(left)?.logical_and(self.eval(right)?)?,
			ASTNodeEnum::Binary {left, op: TokenType::TokenLogicalOr, right} =>
				self.eval(left)?.logical_or(self.eval(right)?)?,

			ASTNodeEnum::Binary {left, op: TokenType::TokenRange, right} => {
				match (self.eval(left)?, self.eval(right)?) {
					(EvaluatorResultWrapper { value: EvaluatorResult::Number(start), row, column },
					EvaluatorResultWrapper { value: EvaluatorResult::Number(end), row: _, column: _ }) =>
						EvaluatorResultWrapper { value: EvaluatorResult::Range(start, end, 1.0), column: column, row: row },

					(EvaluatorResultWrapper { value: EvaluatorResult::Range(start, end, _), row, column },
					EvaluatorResultWrapper { value: EvaluatorResult::Number(step), row: _, column: _ }) =>
						EvaluatorResultWrapper { value: EvaluatorResult::Range(start, end, step), column: column, row: row },

					(EvaluatorResultWrapper { value: _, row, column }, _) =>
						Err(EvaluatorError::IncompatibleOperationType { row, column })?
				}
			},

			ASTNodeEnum::Unary {op: TokenType::TokenPlus, value}
				=> self.eval(value)?,
			ASTNodeEnum::Unary {op: TokenType::TokenMinus, value}
				=> self.eval(value)?.neg()?,

			ASTNodeEnum::Function {name, arg} => {
				match self.scope.clone().get(&name) {
					Some(EvaluatorResultWrapper { value: EvaluatorResult::Function { name: _, args: args_types, static_args, func }, column: _, row: _ }) => {
						let arg = self.eval(arg)?;

						self.call_func(Some(name.as_str()), *func, expr.row, expr.column, arg, args_types.clone(), static_args.as_ref().cloned())?
					},
					_ => return Err(EvaluatorError::UnknownFunction { row: expr.row, column: expr.column, name: name.to_string() })
				}
			},
			ASTNodeEnum::FunctionDefinition { name: Some(name), args, block } => {
				let mut static_args = vec![EvaluatorResultWrapper::ast(*block)];

				let mut args_types = vec![];

				for arg in args {
					static_args.push(EvaluatorResultWrapper::string(arg));

					args_types.push(EvaluatorTypes::Everything);
				}

				let result = EvaluatorResultWrapper::function(Some(name.clone()), EvaluatorTypes::Tuple(args_types), Some(static_args), Self::user_func);

				self.scope.insert(name, result.clone());

				result
			},

			ASTNodeEnum::FunctionDefinition { name: None, args, block } => {
				let mut static_args = vec![EvaluatorResultWrapper::ast(*block)];

				let mut args_types = vec![];

				for arg in args {
					static_args.push(EvaluatorResultWrapper::string(arg));

					args_types.push(EvaluatorTypes::Everything);
				}

				EvaluatorResultWrapper::function(None, EvaluatorTypes::Tuple(args_types), Some(static_args), Self::user_func)
			},

			x => todo!("{}", x)
		})
	}
}
