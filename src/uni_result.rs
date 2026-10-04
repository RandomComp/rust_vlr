use std::{fmt::Display, iter::zip};

use crate::{lexer::TokenType, parser::{ASTNode, ASTNodeEnum}};

#[derive(Debug, thiserror::Error)]
pub enum UniResultError {
	#[error("incompatible operation")]
	IncompatibleOperationType,
	#[error("Unknown operation '{0}'")]
	UnknownOperation(TokenType),
	#[error("Cannot convert from {0:?} to UniResult")]
	CannotConvertToUniResult(ASTNodeEnum),
	#[error("Cannot convert to {0:?} from UniResult")]
	CannotConvertFromUniResult(UniResult),
}

#[derive(Debug, Clone, PartialEq)]
pub enum UniResult {
	ArgsEnd,
	None,
	Boolean(bool),
	Number(f64),
	String(String),
	Tuple(Vec<UniResult>),
	Range { start: f64, end: f64, step: f64 },
}

impl TryFrom<UniResult> for ASTNodeEnum {
	type Error = UniResultError;

	fn try_from(value: UniResult) -> Result<Self, Self::Error> {
		let result = match value {
			UniResult::None => ASTNodeEnum::None,
			UniResult::Boolean(val) => ASTNodeEnum::Boolean(val),
			UniResult::Number(val) => ASTNodeEnum::Number(val),
			UniResult::String(val) => ASTNodeEnum::String(val),
			UniResult::Tuple(values) => {
				let result: Result<Vec<ASTNode>, _> = values.into_iter().map(TryInto::try_into).collect();

				ASTNodeEnum::Tuple(result?)
			},
			v => return Err(Self::Error::CannotConvertFromUniResult(v)),
		};

		Ok(result)
	}
}

impl TryFrom<UniResult> for ASTNode {
	type Error = UniResultError;

	fn try_from(value: UniResult) -> Result<Self, Self::Error> {
		Ok(Self { value: value.try_into()?, row: 0, column: 0 })
	}
}

impl TryFrom<ASTNodeEnum> for UniResult {
	type Error = UniResultError;

	fn try_from(value: ASTNodeEnum) -> Result<Self, Self::Error> {
		let result = match value {
			ASTNodeEnum::None => Self::None,
			ASTNodeEnum::Boolean(val) => Self::Boolean(val),
			ASTNodeEnum::Number(val) => Self::Number(val),
			ASTNodeEnum::String(val) => Self::String(val),
			ASTNodeEnum::Tuple(values) => {
				let result: Result<Vec<Self>, _> = values.into_iter().map(TryInto::try_into).collect();

				Self::Tuple(result?)
			},
			v => return Err(Self::Error::CannotConvertToUniResult(v)),
		};

		Ok(result)
	}
}

impl TryFrom<ASTNode> for UniResult {
	type Error = UniResultError;

	fn try_from(val: ASTNode) -> Result<Self, Self::Error> {
		val.value.try_into()
	}
}

pub fn calc_binary_by_op(left: &UniResult, right: &UniResult, op: &TokenType) -> Result<UniResult, UniResultError> {
	match op {
		TokenType::TokenPlus => left.add(right),
		TokenType::TokenMinus => left.sub(right),
		TokenType::TokenMultiply => left.mul(right),
		TokenType::TokenPow => left.pow(right),
		TokenType::TokenDivide => left.div(right),
		TokenType::TokenEquals => left.eq(right),
		TokenType::TokenNotEquals => left.neq(right),
		TokenType::TokenGreat => left.gt(right),
		TokenType::TokenLess => left.lt(right),
		_ => Err(UniResultError::UnknownOperation(op.to_owned())),
	}
}

impl Display for UniResult {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			UniResult::ArgsEnd => write!(f, "<args end>"),
			UniResult::None => write!(f, "None"),
			UniResult::Boolean(x) => write!(f, "{x}"),
			UniResult::Number(x) => write!(f, "{x}"),
			UniResult::String(x) => write!(f, "\"{x}\""),
			UniResult::Tuple(values) => {
				write!(f, "(")?;

				for (i, value) in values.iter().enumerate() {
					if i > 0 {
						write!(f, ", ")?;
					}

					write!(f, "{value}")?;
				}

				write!(f, ")")
			},
			UniResult::Range { start, end, step } => {
				write!(f, "{start}..{end}..{step}")
			},
		}
	}
}

impl UniResult {
	pub fn add(&self, right: &Self) -> Result<Self, UniResultError> {
		match (self, right) {
			(UniResult::Number(a), &UniResult::Number(b))
				=> Ok(UniResult::Number(a + b)),
			(UniResult::Tuple(values), b @ UniResult::Number(..)) |
			(b @ UniResult::Number(..), UniResult::Tuple(values)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(value.add(b)?);
				}

				Ok(result.into())
			},
			(UniResult::Tuple(values_a), UniResult::Tuple(values_b)) => {
				let mut result = Vec::new();

				for (a, b) in zip(values_a, values_b) {
					result.push(a.add(b)?);
				}

				Ok(result.into())
			},
			_ => Err(UniResultError::IncompatibleOperationType),
		}
	}
	pub fn sub(&self, right: &Self) -> Result<Self, UniResultError> {
		match (self, right) {
			(UniResult::Number(a), &UniResult::Number(b))
				=> Ok(UniResult::Number(a - b)),
			(UniResult::Tuple(values), b @ UniResult::Number(..)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(value.sub(b)?);
				}

				Ok(result.into())
			},
			(b @ UniResult::Number(..), UniResult::Tuple(values)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(b.clone().sub(value)?);
				}

				Ok(result.into())
			},
			(UniResult::Tuple(values_a), UniResult::Tuple(values_b)) => {
				let mut result = Vec::new();

				for (a, b) in zip(values_a, values_b) {
					result.push(a.sub(b)?);
				}

				Ok(result.into())
			},
			_ => Err(UniResultError::IncompatibleOperationType),
		}
	}
	pub fn mul(&self, right: &Self) -> Result<Self, UniResultError> {
		match (self, right) {
			(UniResult::Number(a), UniResult::Number(b))
				=> Ok(UniResult::Number(a * b)),
			(UniResult::Tuple(values), &UniResult::Number(b)) |
			(&UniResult::Number(b), UniResult::Tuple(values)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(value.mul(&b.into())?);
				}

				Ok(result.into())
			},

			(UniResult::Tuple(values_a), UniResult::Tuple(values_b)) => {
				let mut result = Vec::new();

				for (a, b) in zip(values_a, values_b) {
					result.push(a.mul(b)?);
				}

				Ok(result.into())
			},
			_ => Err(UniResultError::IncompatibleOperationType),
		}
	}
	pub fn div(&self, right: &Self) -> Result<Self, UniResultError> {
		match (self, right) {
			(UniResult::Number(a), &UniResult::Number(b))
				=> Ok(UniResult::Number(a / b)),
			(UniResult::Tuple(values), b @ UniResult::Number(..)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(value.div(b)?);
				}

				Ok(result.into())
			},
			(b @ UniResult::Number(..), UniResult::Tuple(values)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(b.clone().div(value)?);
				}

				Ok(result.into())
			},
			(UniResult::Tuple(values_a), UniResult::Tuple(values_b)) => {
				let mut result = Vec::new();

				for (a, b) in zip(values_a, values_b) {
					result.push(a.div(b)?);
				}

				Ok(result.into())
			},
			_ => Err(UniResultError::IncompatibleOperationType),
		}
	}
	pub fn pow(&self, right: &Self) -> Result<Self, UniResultError> {
		match (self, right) {
			(UniResult::Number(a), &UniResult::Number(b))
				=> Ok(UniResult::Number(a.powf(b))),
			(UniResult::Tuple(values), b @ UniResult::Number(..)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(value.pow(b)?);
				}

				Ok(result.into())
			},
			(b @ UniResult::Number(..), UniResult::Tuple(values)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(b.clone().pow(value)?);
				}

				Ok(result.into())
			},
			(UniResult::Tuple(values_a), UniResult::Tuple(values_b)) => {
				let mut result = Vec::new();

				for (a, b) in zip(values_a, values_b) {
					result.push(a.pow(b)?);
				}

				Ok(result.into())
			},
			_ => Err(UniResultError::IncompatibleOperationType),
		}
	}

	pub fn eq(&self, right: &Self) -> Result<Self, UniResultError> {
		match (self, right) {
			(&UniResult::Number(a), &UniResult::Number(b))
				=> Ok(UniResult::Boolean((a - b).abs() < f64::EPSILON)),
			(UniResult::String(a), UniResult::String(b))
				=> Ok(UniResult::Boolean(a == b)),
			(b @ UniResult::Number(..), UniResult::Tuple(values)) |
			(UniResult::Tuple(values), b @ UniResult::Number(..)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(value.eq(b)?);
				}

				Ok(result.into())
			},
			(UniResult::Tuple(values_a), UniResult::Tuple(values_b)) => {
				let mut result = Vec::new();

				for (a, b) in zip(values_a, values_b) {
					result.push(a.eq(b)?);
				}

				Ok(result.into())
			},
			_ => Err(UniResultError::IncompatibleOperationType),
		}
	}

	pub fn neq(&self, right: &Self) -> Result<Self, UniResultError> {
		match (self, right) {
			(&UniResult::Number(a), &UniResult::Number(b))
				=> Ok(UniResult::Boolean((a - b).abs() > f64::EPSILON)),
			(UniResult::String(a), UniResult::String(b))
				=> Ok(UniResult::Boolean(a != b)),
			(b @ UniResult::Number(..), UniResult::Tuple(values)) |
			(UniResult::Tuple(values), b @ UniResult::Number(..)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(value.eq(b)?);
				}

				Ok(result.into())
			},
			(UniResult::Tuple(values_a), UniResult::Tuple(values_b)) => {
				let mut result = Vec::new();

				for (a, b) in zip(values_a, values_b) {
					result.push(a.eq(b)?);
				}

				Ok(result.into())
			},
			_ => Err(UniResultError::IncompatibleOperationType),
		}
	}

	pub fn lt(&self, right: &Self) -> Result<Self, UniResultError> {
		match (self, right) {
			(&UniResult::Number(a), &UniResult::Number(b))
				=> Ok(UniResult::Boolean(a < b)),
			(UniResult::String(a), UniResult::String(b))
				=> Ok(UniResult::Boolean(a < b)),
			(b @ UniResult::Number(..), UniResult::Tuple(values)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(b.lt(value)?);
				}

				Ok(result.into())
			},
			(UniResult::Tuple(values), b @ UniResult::Number(..)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(value.lt(b)?);
				}

				Ok(result.into())
			},
			(UniResult::Tuple(values_a), UniResult::Tuple(values_b)) => {
				let mut result = Vec::new();

				for (a, b) in zip(values_a, values_b) {
					result.push(a.lt(b)?);
				}

				Ok(result.into())
			},
			_ => Err(UniResultError::IncompatibleOperationType),
		}
	}

	pub fn gt(&self, right: &Self) -> Result<Self, UniResultError> {
		match (self, right) {
			(&UniResult::Number(a), &UniResult::Number(b))
				=> Ok(UniResult::Boolean(a > b)),
			(UniResult::String(a), UniResult::String(b))
				=> Ok(UniResult::Boolean(a > b)),
			(b @ UniResult::Number(..), UniResult::Tuple(values)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(b.gt(value)?);
				}

				Ok(result.into())
			},
			(UniResult::Tuple(values), b @ UniResult::Number(..)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(value.gt(b)?);
				}

				Ok(result.into())
			},
			(UniResult::Tuple(values_a), UniResult::Tuple(values_b)) => {
				let mut result = Vec::new();

				for (a, b) in zip(values_a, values_b) {
					result.push(a.gt(b)?);
				}

				Ok(result.into())
			},
			_ => Err(UniResultError::IncompatibleOperationType),
		}
	}

	pub fn neg(&self) -> Result<Self, UniResultError> {
		match self {
			&UniResult::Number(x)
				=> Ok(UniResult::Number(-x)),
			UniResult::Tuple(values) => {
				let mut result = Vec::new();

				for value in values {
					result.push(value.neg()?);
				}

				Ok(result.into())
			},
			_ => Err(UniResultError::IncompatibleOperationType),
		}
	}

	pub fn not(&self) -> Result<Self, UniResultError> {
		match self {
			&UniResult::Boolean(x)
				=> Ok(UniResult::Boolean(!x)),
			UniResult::Tuple(values) => {
				let mut result = Vec::new();

				for value in values {
					result.push(value.not()?);
				}

				Ok(result.into())
			},
			_ => Err(UniResultError::IncompatibleOperationType),
		}
	}

	pub fn to_bool(&self) -> Result<bool, UniResultError> {
		match self {
			&UniResult::Boolean(x) => Ok(x),
			_ => Err(UniResultError::IncompatibleOperationType),
		}
	}
}

impl From<bool> for UniResult {
	fn from(value: bool) -> Self {
		UniResult::Boolean(value)
	}
}

impl From<f64> for UniResult {
	fn from(value: f64) -> Self {
		UniResult::Number(value)
	}
}

impl From<String> for UniResult {
	fn from(value: String) -> Self {
		UniResult::String(value)
	}
}

impl From<Vec<UniResult>> for UniResult {
	fn from(values: Vec<UniResult>) -> Self {
		UniResult::Tuple(values)
	}
}

impl From<UniResult> for f64 {
	fn from(value: UniResult) -> f64 {
		if let UniResult::Number(x) = value {
			x
		} else {
			panic!("Using 'into' on incompatible type {value:?}")
		}
	}
}

impl From<UniResult> for String {
	fn from(value: UniResult) -> String {
		if let UniResult::String(x) = value {
			x
		} else {
			panic!("Using 'into' on incompatible type {value:?}")
		}
	}
}

impl From<UniResult> for Vec<UniResult> {
	fn from(value: UniResult) -> Vec<UniResult> {
		if let UniResult::Tuple(x) = value {
			x
		} else {
			panic!("Using 'into' on incompatible type {value:?}")
		}
	}
}
