use std::{fmt::Display, iter::zip};

use crate::{parser::{BinaryOp, RawASTNode, UnaryOp}, res_with_pos, wrapper::{ASTNode, UniResult, UniResultError}};

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum RawUniResultError {
	#[error("Incompatible operation")]
	IncompatibleOperationType,
	#[error("Use '{0}' on tuple is ambigous, use all() or any() instead")]
	LogicalOperationOnTupleAmbigous(BinaryOp),
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
			BinaryOp::Plus 			=> Self::add(left, right),
			BinaryOp::Minus 		=> Self::sub(left, right),
			BinaryOp::Multiply 		=> Self::mul(left, right),
			BinaryOp::Pow 			=> Self::pow(left, right),
			BinaryOp::Divide 		=> Self::div(left, right),
			BinaryOp::Equals 		=> Self::eq(left, right),
			BinaryOp::Great 		=> Self::gt(left, right),
			BinaryOp::Less 			=> Self::lt(left, right),
			BinaryOp::LogicalAnd 	=> Self::log_and(left, right),
			BinaryOp::LogicalOr 	=> Self::log_or(left, right),
			_ => Err(
				UniResultError::from(
					RawUniResultError::UnsupportedOperation(op.to_owned())
				).with_pos_from(left)
			),
		};

		res_with_pos!(result, UniResult, UniResultError, left.span)
	}

	pub fn calc_unary_by_op(left: &UniResult, op: &UnaryOp) -> Result<UniResult, UniResultError> {
		match op {
			UnaryOp::Minus => Self::neg(left),
			UnaryOp::Not => Self::not(left),
		}
	}

	pub fn add(left: &UniResult, right: &UniResult) -> Result<UniResult, UniResultError> {
		let result = match (&left.val, &right.val) {
			(Self::Float(a), &Self::Float(b))
				=> Ok(Self::Float(a + b)),
			(Self::Tuple(values), Self::Float(..)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(Self::add(value, right)?);
				}

				Ok(result.into())
			},
			(Self::Float(..), Self::Tuple(values)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(Self::add(value, left)?);
				}

				Ok(result.into())
			},
			(Self::Tuple(values_a), Self::Tuple(values_b)) => {
				let mut result = Vec::new();

				for (a, b) in zip(values_a, values_b) {
					result.push(Self::add(a, b)?);
				}

				Ok(result.into())
			},
			_ => Err(RawUniResultError::IncompatibleOperationType),
		};

		res_with_pos!(result, UniResult, UniResultError, left.span)
	}
	pub fn sub(left: &UniResult, right: &UniResult) -> Result<UniResult, UniResultError> {
		let result = match (&left.val, &right.val) {
			(Self::Float(a), &Self::Float(b))
				=> Ok(Self::Float(a - b)),
			(Self::Tuple(values), Self::Float(..)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(Self::sub(value, right)?);
				}

				Ok(result.into())
			},
			(Self::Float(..), Self::Tuple(values)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(Self::sub(left, value)?);
				}

				Ok(result.into())
			},
			(Self::Tuple(values_a), Self::Tuple(values_b)) => {
				let mut result = Vec::new();

				for (a, b) in zip(values_a, values_b) {
					result.push(Self::add(a, b)?);
				}

				Ok(result.into())
			},
			_ => Err(RawUniResultError::IncompatibleOperationType),
		};

		res_with_pos!(result, UniResult, UniResultError, left.span)
	}
	pub fn pow(left: &UniResult, right: &UniResult) -> Result<UniResult, UniResultError> {
		let result = match (&left.val, &right.val) {
			(Self::Float(a), &Self::Float(b))
				=> Ok(Self::Float(a.powf(b))),
			(Self::Tuple(values), Self::Float(..)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(Self::pow(value, right)?);
				}

				Ok(result.into())
			},
			(Self::Float(..), Self::Tuple(values)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(Self::pow(left, value)?);
				}

				Ok(result.into())
			},
			(Self::Tuple(values_a), Self::Tuple(values_b)) => {
				let mut result = Vec::new();

				for (a, b) in zip(values_a, values_b) {
					result.push(Self::pow(a, b)?);
				}

				Ok(result.into())
			},
			_ => Err(RawUniResultError::IncompatibleOperationType),
		};

		res_with_pos!(result, UniResult, UniResultError, left.span)
	}
	pub fn mul(left: &UniResult, right: &UniResult) -> Result<UniResult, UniResultError> {
		let result = match (&left.val, &right.val) {
			(Self::Float(a), &Self::Float(b))
				=> Ok(Self::Float(a * b)),
			(Self::Tuple(values), Self::Float(..)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(Self::mul(value, right)?);
				}

				Ok(result.into())
			},
			(Self::Float(..), Self::Tuple(values)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(Self::mul(left, value)?);
				}

				Ok(result.into())
			},
			(Self::Tuple(values_a), Self::Tuple(values_b)) => {
				let mut result = Vec::new();

				for (a, b) in zip(values_a, values_b) {
					result.push(Self::mul(a, b)?);
				}

				Ok(result.into())
			},
			_ => Err(RawUniResultError::IncompatibleOperationType),
		};

		res_with_pos!(result, UniResult, UniResultError, left.span)
	}
	pub fn div(left: &UniResult, right: &UniResult) -> Result<UniResult, UniResultError> {
		let result = match (&left.val, &right.val) {
			(Self::Float(a), &Self::Float(b))
				=> Ok(Self::Float(a / b)),
			(Self::Tuple(values), Self::Float(..)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(Self::div(value, right)?);
				}

				Ok(result.into())
			},
			(Self::Float(..), Self::Tuple(values)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(Self::div(left, value)?);
				}

				Ok(result.into())
			},
			(Self::Tuple(values_a), Self::Tuple(values_b)) => {
				let mut result = Vec::new();

				for (a, b) in zip(values_a, values_b) {
					result.push(Self::div(a, b)?);
				}

				Ok(result.into())
			},
			_ => Err(RawUniResultError::IncompatibleOperationType),
		};

		res_with_pos!(result, UniResult, UniResultError, left.span)
	}
	pub fn rem(left: &UniResult, right: &UniResult) -> Result<UniResult, UniResultError> {
		let result = match (&left.val, &right.val) {
			(Self::Float(a), &Self::Float(b))
				=> Ok(Self::Float(a % b)),
			(Self::Tuple(values), Self::Float(..)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(Self::rem(value, right)?);
				}

				Ok(result.into())
			},
			(Self::Float(..), Self::Tuple(values)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(Self::rem(left, value)?);
				}

				Ok(result.into())
			},
			(Self::Tuple(values_a), Self::Tuple(values_b)) => {
				let mut result = Vec::new();

				for (a, b) in zip(values_a, values_b) {
					result.push(Self::rem(a, b)?);
				}

				Ok(result.into())
			},
			_ => Err(RawUniResultError::IncompatibleOperationType),
		};

		res_with_pos!(result, UniResult, UniResultError, left.span)
	}

	pub fn eq(left: &UniResult, right: &UniResult) -> Result<UniResult, UniResultError> {
		let result = match (&left.val, &right.val) {
			(Self::Float(a), &Self::Float(b))
				=> Ok(Self::Boolean((a - b).abs() < f64::EPSILON)),
			(Self::Tuple(values), Self::Float(..)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(Self::eq(value, right)?);
				}

				Ok(result.into())
			},
			(Self::Float(..), Self::Tuple(values)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(Self::eq(left, value)?);
				}

				Ok(result.into())
			},
			(Self::Tuple(values_a), Self::Tuple(values_b)) => {
				let mut result = Vec::new();

				for (a, b) in zip(values_a, values_b) {
					result.push(Self::eq(a, b)?);
				}

				Ok(result.into())
			},
			_ => Err(RawUniResultError::IncompatibleOperationType),
		};

		res_with_pos!(result, UniResult, UniResultError, left.span)
	}
	pub fn lt(left: &UniResult, right: &UniResult) -> Result<UniResult, UniResultError> {
		let result = match (&left.val, &right.val) {
			(&Self::Float(a), &Self::Float(b))
				=> Ok(Self::Boolean(a < b)),
			(Self::Tuple(values), Self::Float(..)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(Self::lt(value, right)?);
				}

				Ok(result.into())
			},
			(Self::Float(..), Self::Tuple(values)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(Self::lt(left, value)?);
				}

				Ok(result.into())
			},
			(Self::Tuple(values_a), Self::Tuple(values_b)) => {
				let mut result = Vec::new();

				for (a, b) in zip(values_a, values_b) {
					result.push(Self::lt(a, b)?);
				}

				Ok(result.into())
			},
			_ => Err(RawUniResultError::IncompatibleOperationType),
		};

		res_with_pos!(result, UniResult, UniResultError, left.span)
	}

	pub fn gt(left: &UniResult, right: &UniResult) -> Result<UniResult, UniResultError> {
		let result = match (&left.val, &right.val) {
			(&Self::Float(a), &Self::Float(b))
				=> Ok(Self::Boolean(a > b)),
			(Self::Tuple(values), Self::Float(..)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(Self::gt(value, right)?);
				}

				Ok(result.into())
			},
			(Self::Float(..), Self::Tuple(values)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(Self::gt(left, value)?);
				}

				Ok(result.into())
			},
			(Self::Tuple(values_a), Self::Tuple(values_b)) => {
				let mut result = Vec::new();

				for (a, b) in zip(values_a, values_b) {
					result.push(Self::gt(a, b)?);
				}

				Ok(result.into())
			},
			_ => Err(RawUniResultError::IncompatibleOperationType),
		};

		res_with_pos!(result, UniResult, UniResultError, left.span)
	}

	pub fn log_and(left: &UniResult, right: &UniResult) -> Result<UniResult, UniResultError> {
		let result = match (&left.val, &right.val) {
			(&Self::Boolean(a), &Self::Boolean(b))
				=> Ok(Self::Boolean(a && b)),
			(Self::Tuple(..) | Self::Float(..), Self::Tuple(..) | Self::Float(..)) => {
				Err(RawUniResultError::LogicalOperationOnTupleAmbigous(BinaryOp::LogicalAnd))
			},
			_ => Err(RawUniResultError::IncompatibleOperationType),
		};

		res_with_pos!(result, UniResult, UniResultError, left.span)
	}

	pub fn log_or(left: &UniResult, right: &UniResult) -> Result<UniResult, UniResultError> {
		let result = match (&left.val, &right.val) {
			(&Self::Float(a), &Self::Float(b))
				=> Ok(Self::Boolean(a > b)),
			(Self::Tuple(values), Self::Float(..)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(Self::gt(value, right)?);
				}

				Ok(result.into())
			},
			(Self::Float(..), Self::Tuple(values)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(Self::gt(left, value)?);
				}

				Ok(result.into())
			},
			(Self::Tuple(values_a), Self::Tuple(values_b)) => {
				let mut result = Vec::new();

				for (a, b) in zip(values_a, values_b) {
					result.push(Self::gt(a, b)?);
				}

				Ok(result.into())
			},
			_ => Err(RawUniResultError::IncompatibleOperationType),
		};

		res_with_pos!(result, UniResult, UniResultError, left.span)
	}

	pub fn neg(value: &UniResult) -> Result<UniResult, UniResultError> {
		let result = match &value.val {
			&Self::Float(b)
				=> Ok(Self::Float(-b)),
			Self::Tuple(values) => {
				let mut result = Vec::new();

				for value in values {
					result.push(Self::neg(value)?);
				}

				Ok(result.into())
			},
			_ => Err(RawUniResultError::IncompatibleOperationType),
		};

		res_with_pos!(result, UniResult, UniResultError, value.span)
	}
	pub fn not(value: &UniResult) -> Result<UniResult, UniResultError> {
		let result = match &value.val {
			&Self::Boolean(b)
				=> Ok(Self::Boolean(!b)),
			Self::Tuple(values) => {
				let mut result = Vec::new();

				for value in values {
					result.push(Self::not(value)?);
				}

				Ok(result.into())
			},
			_ => Err(RawUniResultError::IncompatibleOperationType),
		};

		res_with_pos!(result, UniResult, UniResultError, value.span)
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

impl From<RawUniResult> for bool {
	fn from(value: RawUniResult) -> bool {
		let RawUniResult::Boolean(x) = value else {
			panic!("Using 'into' on incompatible type {value:?}")
		};

		x
	}
}

impl From<RawUniResult> for u32 {
	fn from(value: RawUniResult) -> u32 {
		let RawUniResult::NumberU32(x) = value else {
			panic!("Using 'into' on incompatible type {value:?}")
		};

		x
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
