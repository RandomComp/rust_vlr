use std::{fmt::Display, iter::zip};

use crate::{parser::{BinaryOp, RawASTNode, UnaryOp}, res_with_pos, val_with_pos, wrapper::{ASTNode, UniResult, UniResultError}};

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum RawUniResultError {
	#[error("Incompatible operation")]
	IncompatibleOperationType,
	#[error("Unsupported operation '{0}'")]
	UnsupportedOperation(BinaryOp),
	#[error("Cannot convert from {0:?} to UniResult")]
	CannotConvertToUniResult(RawASTNode),
	#[error("Cannot convert to {0:?} from UniResult")]
	CannotConvertFromUniResult(RawUniResult),
}

#[derive(Debug, Clone, PartialEq)]
pub enum RawUniResult {
	None,
	Boolean(bool),
	NumberU32(u32),
	Float(f64),
	String(String),
	Tuple(Vec<UniResult>),
	Range { start: f64, end: f64, step: f64 },
}

impl TryFrom<UniResult> for ASTNode {
	type Error = UniResultError;

	fn try_from(value: UniResult) -> Result<Self, Self::Error> {
		let span = value.span;

		let result = match value.val {
			RawUniResult::None =>
				RawASTNode::None,
			RawUniResult::Boolean(val) =>
				RawASTNode::Boolean(val),
			RawUniResult::Float(val) =>
				RawASTNode::Float(val),
			RawUniResult::String(val) =>
				RawASTNode::String(val),
			RawUniResult::Tuple(values) => {
				let result: Result<Vec<ASTNode>, _> = values.into_iter().map(TryInto::try_into).collect();

				RawASTNode::Tuple(result?)
			},
			v => return Err(
				Self::Error::from(
					RawUniResultError::CannotConvertFromUniResult(v)
				).with_pos(span)
			),
		};

		Ok(
			Self::from(result).with_pos(span)
		)
	}
}

impl TryFrom<ASTNode> for UniResult {
	type Error = UniResultError;

	fn try_from(value: ASTNode) -> Result<Self, Self::Error> {
		use RawUniResult as RawSelf;

		let span = value.span;

		let result = match value.val {
			RawASTNode::None => RawSelf::None,
			RawASTNode::Boolean(val) => RawSelf::Boolean(val),
			RawASTNode::Float(val) => RawSelf::Float(val),
			RawASTNode::String(val) => RawSelf::String(val),
			RawASTNode::Tuple(values) => {
				let result: Result<Vec<Self>, _> = values.into_iter().map(TryInto::try_into).collect();

				RawSelf::Tuple(result?)
			},
			v => return Err(
				Self::Error::from(
					RawUniResultError::CannotConvertToUniResult(v)
				).with_pos(span)
			),
		};

		Ok(
			Self::from(result).with_pos(span)
		)
	}
}

impl Display for RawUniResult {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::None => write!(f, "None"),
			Self::Boolean(x) => write!(f, "{x}"),
			Self::NumberU32(x) => write!(f, "{x}"),
			Self::Float(x) => write!(f, "{x}"),
			Self::String(x) => write!(f, "\"{x}\""),
			Self::Tuple(values) => {
				write!(f, "(")?;

				for (i, value) in values.iter().enumerate() {
					if i > 0 {
						write!(f, ", ")?;
					}

					write!(f, "{value}")?;
				}

				write!(f, ")")
			},
			Self::Range { start, end, step } => {
				write!(f, "{start}..{end}..{step}")
			},
		}
	}
}

impl RawUniResult {
	pub fn calc_binary_by_op(left: &UniResult, right: &UniResult, op: &BinaryOp) -> Result<UniResult, UniResultError> {
		let result = match op {
			BinaryOp::Plus 			=> left.add(&right.val),
			BinaryOp::Minus 		=> left.sub(&right.val),
			BinaryOp::Multiply 		=> left.mul(&right.val),
			BinaryOp::Pow 			=> left.pow(&right.val),
			BinaryOp::Divide 		=> left.div(&right.val),
			BinaryOp::Equals 		=> left.val.eq(&right.val),
			BinaryOp::Neq 			=> left.neq(&right.val),
			BinaryOp::Great 		=> left.gt(&right.val),
			BinaryOp::Less 			=> left.lt(&right.val),
			BinaryOp::LogicalAnd 	=> left.log_and(&right.val),
			BinaryOp::LogicalOr 	=> left.log_or(&right.val),
			_ => Err(
				UniResultError::from(
					RawUniResultError::UnsupportedOperation(op.to_owned())
				).with_pos_from(left)
			),
		};

		res_with_pos!(result, UniResult, UniResultError, left.span)
	}

	pub fn calc_unary_by_op(&self, op: &UnaryOp) -> Result<Self, UniResultError> {
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
				=> Ok(Self::Boolean(a == b)),
			(&Self::Float(a), &Self::Float(b))
				=> Ok(Self::Boolean((a - b).abs() < f64::EPSILON)),
			(Self::String(a), Self::String(b))
				=> Ok(Self::Boolean(a == b)),
			(b @ Self::Float(..), Self::Tuple(values)) |
			(Self::Tuple(values), b @ Self::Float(..)) => {
				let mut result = Vec::new();

				for value in values.iter().map(|v| v.val) {
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

impl From<bool> for RawUniResult {
	fn from(value: bool) -> Self {
		RawUniResult::Boolean(value)
	}
}

impl From<u32> for RawUniResult {
	fn from(value: u32) -> Self {
		RawUniResult::NumberU32(value)
	}
}

impl From<f64> for RawUniResult {
	fn from(value: f64) -> Self {
		RawUniResult::Float(value)
	}
}

impl From<String> for RawUniResult {
	fn from(value: String) -> Self {
		RawUniResult::String(value)
	}
}

impl From<Vec<UniResult>> for RawUniResult {
	fn from(values: Vec<UniResult>) -> Self {
		RawUniResult::Tuple(values)
	}
}

impl From<RawUniResult> for f64 {
	fn from(value: RawUniResult) -> f64 {
		let RawUniResult::Float(x) = value else {
			panic!("Using 'into' on incompatible type {value:?}")
		};

		x
	}
}

impl From<RawUniResult> for String {
	fn from(value: RawUniResult) -> String {
		let RawUniResult::String(x) = value else {
			panic!("Using 'into' on incompatible type {value:?}")
		};

		x
	}
}

impl From<RawUniResult> for Vec<UniResult> {
	fn from(value: RawUniResult) -> Vec<UniResult> {
		let RawUniResult::Tuple(x) = value else {
			panic!("Using 'into' on incompatible type {value:?}")
		};

		x
	}
}
