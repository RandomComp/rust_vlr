use std::{fmt::Display, iter::zip};

use crate::parser::{ASTNode, ASTNodeEnum, BinaryOp, UnaryOp};

#[derive(Debug, thiserror::Error)]
pub enum UniResultError {
	#[error("Incompatible operation")]
	IncompatibleOperationType,
	#[error("Unsupported operation '{0}'")]
	UnsupportedOperation(BinaryOp),
	#[error("Cannot convert from {0:?} to UniResult")]
	CannotConvertToUniResult(ASTNodeEnum),
	#[error("Cannot convert to {0:?} from UniResult")]
	CannotConvertFromUniResult(UniResult),
}

#[derive(Debug, Clone, PartialEq)]
pub enum UniResult {
	None,
	Boolean(bool),
	NumberU32(u32),
	Float(f64),
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
			UniResult::Float(val) => ASTNodeEnum::Float(val),
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
			ASTNodeEnum::Float(val) => Self::Float(val),
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

impl Display for UniResult {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			UniResult::None => write!(f, "None"),
			UniResult::Boolean(x) => write!(f, "{x}"),
			UniResult::NumberU32(x) => write!(f, "{x}"),
			UniResult::Float(x) => write!(f, "{x}"),
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
	pub fn calc_binary_by_op(&self, right: &UniResult, op: &BinaryOp) -> Result<UniResult, UniResultError> {
		match op {
			BinaryOp::Plus 			=> self.add(right),
			BinaryOp::Minus 		=> self.sub(right),
			BinaryOp::Multiply 		=> self.mul(right),
			BinaryOp::Pow 			=> self.pow(right),
			BinaryOp::Divide 		=> self.div(right),
			BinaryOp::Equals 		=> self.eq(right),
			BinaryOp::Neq 	=> self.neq(right),
			BinaryOp::Great 		=> self.gt(right),
			BinaryOp::Less 			=> self.lt(right),
			BinaryOp::LogicalAnd 	=> self.log_and(right),
			BinaryOp::LogicalOr 	=> self.log_or(right),
			_ => Err(UniResultError::UnsupportedOperation(op.to_owned())),
		}
	}

	pub fn calc_unary_by_op(&self, op: &UnaryOp) -> Result<UniResult, UniResultError> {
		match op {
			UnaryOp::Minus => self.neg(),
			UnaryOp::Not => self.not(),
		}
	}

	pub fn add(&self, right: &Self) -> Result<Self, UniResultError> {
		match (self, right) {
			(Self::Float(a), &Self::Float(b))
				=> Ok(Self::Float(a + b)),
			(Self::Tuple(values), b @ Self::Float(..)) |
			(b @ Self::Float(..), Self::Tuple(values)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(value.add(b)?);
				}

				Ok(result.into())
			},
			(Self::Tuple(values_a), Self::Tuple(values_b)) => {
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
			(Self::Float(a), &Self::Float(b))
				=> Ok(Self::Float(a - b)),
			(Self::Tuple(values), b @ Self::Float(..)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(value.sub(b)?);
				}

				Ok(result.into())
			},
			(b @ Self::Float(..), Self::Tuple(values)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(b.clone().sub(value)?);
				}

				Ok(result.into())
			},
			(Self::Tuple(values_a), Self::Tuple(values_b)) => {
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
			(Self::Float(a), Self::Float(b))
				=> Ok(Self::Float(a * b)),
			(Self::Tuple(values), &Self::Float(b)) |
			(&Self::Float(b), Self::Tuple(values)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(value.mul(&b.into())?);
				}

				Ok(result.into())
			},

			(Self::Tuple(values_a), Self::Tuple(values_b)) => {
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
			(Self::Float(a), &Self::Float(b))
				=> Ok(Self::Float(a / b)),
			(Self::Tuple(values), b @ Self::Float(..)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(value.div(b)?);
				}

				Ok(result.into())
			},
			(b @ Self::Float(..), Self::Tuple(values)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(b.clone().div(value)?);
				}

				Ok(result.into())
			},
			(Self::Tuple(values_a), Self::Tuple(values_b)) => {
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
			(Self::Float(a), &Self::Float(b))
				=> Ok(Self::Float(a.powf(b))),
			(Self::Tuple(values), b @ Self::Float(..)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(value.pow(b)?);
				}

				Ok(result.into())
			},
			(b @ Self::Float(..), Self::Tuple(values)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(b.clone().pow(value)?);
				}

				Ok(result.into())
			},
			(Self::Tuple(values_a), Self::Tuple(values_b)) => {
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
			(&Self::Boolean(a), &Self::Boolean(b))
				=> Ok(UniResult::Boolean(a == b)),
			(&Self::Float(a), &Self::Float(b))
				=> Ok(Self::Boolean((a - b).abs() < f64::EPSILON)),
			(Self::String(a), Self::String(b))
				=> Ok(Self::Boolean(a == b)),
			(b @ Self::Float(..), Self::Tuple(values)) |
			(Self::Tuple(values), b @ Self::Float(..)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(value.eq(b)?);
				}

				Ok(result.into())
			},
			(Self::Tuple(values_a), Self::Tuple(values_b)) => {
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
			(&Self::Boolean(a), &Self::Boolean(b))
				=> Ok(Self::Boolean(a != b)),
			(&Self::Float(a), &Self::Float(b))
				=> Ok(Self::Boolean((a - b).abs() > f64::EPSILON)),
			(Self::String(a), Self::String(b))
				=> Ok(Self::Boolean(a != b)),
			(b @ Self::Float(..), Self::Tuple(values)) |
			(Self::Tuple(values), b @ Self::Float(..)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(value.eq(b)?);
				}

				Ok(result.into())
			},
			(Self::Tuple(values_a), Self::Tuple(values_b)) => {
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
			(&Self::Float(a), &Self::Float(b))
				=> Ok(Self::Boolean(a < b)),
			(Self::String(a), Self::String(b))
				=> Ok(Self::Boolean(a < b)),
			(b @ Self::Float(..), Self::Tuple(values)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(b.lt(value)?);
				}

				Ok(result.into())
			},
			(Self::Tuple(values), b @ Self::Float(..)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(value.lt(b)?);
				}

				Ok(result.into())
			},
			(Self::Tuple(values_a), Self::Tuple(values_b)) => {
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
			(&Self::Float(a), &Self::Float(b))
				=> Ok(Self::Boolean(a > b)),
			(Self::String(a), Self::String(b))
				=> Ok(Self::Boolean(a > b)),
			(b @ Self::Float(..), Self::Tuple(values)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(b.gt(value)?);
				}

				Ok(result.into())
			},
			(Self::Tuple(values), b @ Self::Float(..)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(value.gt(b)?);
				}

				Ok(result.into())
			},
			(Self::Tuple(values_a), Self::Tuple(values_b)) => {
				let mut result = Vec::new();

				for (a, b) in zip(values_a, values_b) {
					result.push(a.gt(b)?);
				}

				Ok(result.into())
			},
			_ => Err(UniResultError::IncompatibleOperationType),
		}
	}

	pub fn log_and(&self, right: &Self) -> Result<Self, UniResultError> {
		match (self, right) {
			(&Self::Boolean(a), &Self::Boolean(b)) =>
				Ok(Self::Boolean(a && b)),
			(x @ Self::Boolean(_), Self::Tuple(values)) |
			(Self::Tuple(values), x @ Self::Boolean(_)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(value.log_and(x)?);
				}

				Ok(result.into())
			},
			(Self::Tuple(values_a), Self::Tuple(values_b)) => {
				let mut result = Vec::new();

				for (a, b) in zip(values_a, values_b) {
					result.push(a.log_and(b)?);
				}

				Ok(result.into())
			},
			_ => Err(UniResultError::IncompatibleOperationType),
		}
	}

	pub fn log_or(&self, right: &Self) -> Result<Self, UniResultError> {
		match (self, right) {
			(&Self::Boolean(a), &Self::Boolean(b)) =>
				Ok(Self::Boolean(a || b)),
			(x @ Self::Boolean(_), Self::Tuple(values)) |
			(Self::Tuple(values), x @ Self::Boolean(_)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(value.log_or(x)?);
				}

				Ok(result.into())
			},
			(Self::Tuple(values_a), Self::Tuple(values_b)) => {
				let mut result = Vec::new();

				for (a, b) in zip(values_a, values_b) {
					result.push(a.log_or(b)?);
				}

				Ok(result.into())
			},
			_ => Err(UniResultError::IncompatibleOperationType),
		}
	}

	pub fn neg(&self) -> Result<Self, UniResultError> {
		match self {
			&Self::Float(x)
				=> Ok(Self::Float(-x)),
			Self::Tuple(values) => {
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
			&Self::Boolean(x)
				=> Ok(Self::Boolean(!x)),
			Self::Tuple(values) => {
				let mut result = Vec::new();

				for value in values {
					result.push(value.not()?);
				}

				Ok(result.into())
			},
			_ => Err(UniResultError::IncompatibleOperationType),
		}
	}

	pub fn to_u32(&self) -> Result<u32, UniResultError> {
		if let &Self::NumberU32(x) = self {
			Ok(x)
		} else {
			Err(UniResultError::IncompatibleOperationType)
		}
	}

	pub fn to_bool(&self) -> Result<bool, UniResultError> {
		if let &Self::Boolean(x) = self {
			Ok(x)
		} else {
			Err(UniResultError::IncompatibleOperationType)
		}
	}
}

impl From<bool> for UniResult {
	fn from(value: bool) -> Self {
		UniResult::Boolean(value)
	}
}

impl From<u32> for UniResult {
	fn from(value: u32) -> Self {
		UniResult::NumberU32(value)
	}
}

impl From<f64> for UniResult {
	fn from(value: f64) -> Self {
		UniResult::Float(value)
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
		let UniResult::Float(x) = value else {
			panic!("Using 'into' on incompatible type {value:?}")
		};

		x
	}
}

impl From<UniResult> for String {
	fn from(value: UniResult) -> String {
		let UniResult::String(x) = value else {
			panic!("Using 'into' on incompatible type {value:?}")
		};

		x
	}
}

impl From<UniResult> for Vec<UniResult> {
	fn from(value: UniResult) -> Vec<UniResult> {
		let UniResult::Tuple(x) = value else {
			panic!("Using 'into' on incompatible type {value:?}")
		};

		x
	}
}
