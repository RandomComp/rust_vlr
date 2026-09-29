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
	// #[error("line {row} at {column}: too few arguments for function {function}")]
	// TooFewArgumentsForFunction {row: usize, column: usize, function: &'static str},
	// #[error("line {row} at {column}: too many arguments for function {function}")]
	// TooManyArgumentsForFunction {row: usize, column: usize, function: &'static str},
	#[error("line {row} at {column}: unknown function {name}")]
	UnknownFunction {row: usize, column: usize, name: String},
	#[error("formatting error: {0}")]
	FormatError (#[from] fmt::Error),
	// #[error("line {row} at {column}: division by zero")]
	// DivisionByZero {row: usize, column: usize},
}

#[derive(Clone, PartialEq)]
pub enum EvaluatorTypes {
	None,
	Type,
	Boolean,
	Number,
	String,
	Range,
	// Tuple(Vec<EvaluatorTypes>),
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
			// EvaluatorTypes::Tuple(types) => {
			// 	write!(f, "(")?;

			// 	for (i, value) in types.iter().enumerate() {
			// 		if i > 0 {
			// 			write!(f, ", ")?;
			// 		}

			// 		write!(f, "{}", value)?;
			// 	}

			// 	write!(f, ")")
			// },
			EvaluatorTypes::TypedTuple(tuple_type) => {
				write!(f, "typed_tuple({})", tuple_type)
			},
			EvaluatorTypes::Function => write!(f, "function"),
			EvaluatorTypes::Type => write!(f, "type"),
			EvaluatorTypes::Everything => write!(f, "..."),
		}
	}
}

#[derive(Clone)]
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
		name: String,
		args: EvaluatorTypes,
		func: fn(row: usize, column: usize, args: EvaluatorResultWrapper) -> Result<EvaluatorResultWrapper, EvaluatorError>,
	},
}

#[derive(Clone)]
pub struct EvaluatorResultWrapper {
	pub value: EvaluatorResult,
	pub row: usize, pub column: usize,
}

impl EvaluatorResult {
	pub fn to_str(&self, f: &mut String) -> fmt::Result {
		match self {
			EvaluatorResult::None => write!(f, "()"),
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
			EvaluatorResult::Function { name, args, func: _ } => {
				write!(f, "function {}({})", name, args.to_string())
			}
			EvaluatorResult::Type(x) => write!(f, "{}", x),
		}
	}

	fn get_type(&self) -> EvaluatorResultWrapper {
		match self {
			EvaluatorResult::None => {
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
			EvaluatorResult::Function { name: _, args: _, func: _ } => {
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
	// fn boolean(value: bool) -> Self {
	// 	Self::from_result(EvaluatorResult::Boolean(value))
	// }
	fn number(value: f64) -> Self {
		Self::from_result(EvaluatorResult::Number(value))
	}
	// fn string(value: String) -> Self {
	// 	Self::from_result(EvaluatorResult::String(value))
	// }
	fn tuple(value: Vec<EvaluatorResultWrapper>) -> Self {
		Self::from_result(EvaluatorResult::Tuple(value))
	}
	fn function(name: String, args: EvaluatorTypes, func: fn(usize, usize, EvaluatorResultWrapper) -> Result<EvaluatorResultWrapper, EvaluatorError>) -> Self {
		Self::from_result(EvaluatorResult::Function { name, args, func })
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
	// fn boolean(value: bool) -> Self {
	// 	Self::from_result(EvaluatorResult::Boolean(value))
	// }
	// fn number_with_pos(row: usize, column: usize, value: f64) -> Self {
	// 	Self::from_result_with_pos(row, column, EvaluatorResult::Number(value))
	// }
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
			EvaluatorResultWrapper { value: EvaluatorResult::Number(b), row: _, column: _ } )
				=> EvaluatorResult::Number(a % b),
			(EvaluatorResultWrapper { value: EvaluatorResult::Tuple(values), row: _, column: _ }, b) |
			(b, EvaluatorResultWrapper { value: EvaluatorResult::Tuple(values), row: _, column: _ }) => {
				let mut result = Vec::new();

				for value in values {
					result.push(value.remainder(b.clone())?);
				}

				EvaluatorResult::Tuple(result)
			},
			_ => return Err(EvaluatorError::IncompatibleOperationType { row: row, column: column }),
		};

		Ok(EvaluatorResultWrapper {value: result, column: column, row: row})
	}

	fn equals(self, rhs: Self) -> Result<Self, EvaluatorError> {
		let result = match (self.value, rhs.value) {
			(EvaluatorResult::Number(a), EvaluatorResult::Number(b))
				=> EvaluatorResult::Boolean(a == b),
			(EvaluatorResult::String(a), EvaluatorResult::String(b))
				=> EvaluatorResult::Boolean(a == b),
			(EvaluatorResult::Boolean(a), EvaluatorResult::Boolean(b))
				=> EvaluatorResult::Boolean(a == b),
			(EvaluatorResult::Type(a), EvaluatorResult::Type(b))
				=> EvaluatorResult::Boolean(a == b),
			(EvaluatorResult::Tuple(values_a), EvaluatorResult::Tuple(values_b)) => {
				let mut result = Vec::new();

				for (value_a, value_b) in zip(values_a, values_b) {
					result.push(value_a.equals(value_b)?);
				}

				EvaluatorResult::Tuple(result)
			},
			_ => return Err(EvaluatorError::IncompatibleOperationType { row: self.row, column: self.column }),
		};

		Ok(EvaluatorResultWrapper {value: result, column: self.column, row: self.row})
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
		let mut scope: HashMap<String, EvaluatorResultWrapper> = HashMap::new();

		scope.insert(String::from("sqrt"), EvaluatorResultWrapper::function(String::from("sqrt"), EvaluatorTypes::TypedTuple(Box::new(EvaluatorTypes::Number)), Self::sqrt));
		scope.insert(String::from("sin"), EvaluatorResultWrapper::function(String::from("sin"), EvaluatorTypes::TypedTuple(Box::new(EvaluatorTypes::Number)), Self::sin));
		scope.insert(String::from("cos"), EvaluatorResultWrapper::function(String::from("cos"), EvaluatorTypes::TypedTuple(Box::new(EvaluatorTypes::Number)), Self::cos));
		scope.insert(String::from("print"), EvaluatorResultWrapper::function(String::from("print"), EvaluatorTypes::Everything, Self::print));
		scope.insert(String::from("type"), EvaluatorResultWrapper::function(String::from("type"), EvaluatorTypes::Everything, Self::type_name));
		scope.insert(String::from("repr"), EvaluatorResultWrapper::function(String::from("repr"), EvaluatorTypes::Everything, Self::repr));
		scope.insert(String::from("str"), EvaluatorResultWrapper::function(String::from("str"), EvaluatorTypes::Everything, Self::to_str));
		scope.insert(String::from("tuple"), EvaluatorResultWrapper::function(String::from("tuple"), EvaluatorTypes::Everything, Self::to_tuple));

		Self {scope: scope}
	}

	fn sqrt(_row: usize, _column: usize, arg: EvaluatorResultWrapper) -> Result<EvaluatorResultWrapper, EvaluatorError> {
		arg.sqrt()
	}

	fn sin(_row: usize, _column: usize, arg: EvaluatorResultWrapper) -> Result<EvaluatorResultWrapper, EvaluatorError> {
		arg.sin()
	}

	fn cos(_row: usize, _column: usize, arg: EvaluatorResultWrapper) -> Result<EvaluatorResultWrapper, EvaluatorError> {
		arg.cos()
	}

	fn print(_row: usize, _column: usize, arg: EvaluatorResultWrapper) -> Result<EvaluatorResultWrapper, EvaluatorError> {
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

	fn type_name(_row: usize, _column: usize, arg: EvaluatorResultWrapper) -> Result<EvaluatorResultWrapper, EvaluatorError> {
		Ok(arg.value.get_type())
	}

	fn repr(_row: usize, _column: usize, arg: EvaluatorResultWrapper) -> Result<EvaluatorResultWrapper, EvaluatorError> {
		let mut result = String::new();

		arg.value.repr(&mut result)?;

		Ok(EvaluatorResultWrapper::from_result(EvaluatorResult::String(result)))
	}

	fn to_str(_row: usize, _column: usize, arg: EvaluatorResultWrapper) -> Result<EvaluatorResultWrapper, EvaluatorError> {
		let mut result = String::new();

		arg.value.to_str(&mut result)?;

		Ok(EvaluatorResultWrapper::from_result(EvaluatorResult::String(result)))
	}

	fn to_tuple(row: usize, column: usize, arg: EvaluatorResultWrapper) -> Result<EvaluatorResultWrapper, EvaluatorError> {
		Ok(arg.to_tuple(row, column)?)
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
			ASTNodeEnum::Assignment {name, value} => {
				let result = self.eval(value)?;

				self.scope.insert(name, result.clone());

				result
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
				match self.scope.get(name.as_str()) {
					Some(EvaluatorResultWrapper { value: EvaluatorResult::Function { name: _, args: _, func }, column: _, row: _ }) =>
						func(expr.row, expr.column, self.eval(arg)?)?,
					_ => return Err(EvaluatorError::UnknownFunction { row: expr.row, column: expr.column, name: name })
				}
			}

			x => todo!("{}", x)
		})
	}
}
