use std::fmt::{self, Display};

use crate::{compiler::parser::{ASTNode, BinaryOp, UnaryOp}, res_with_pos, wrapper::Spanned};

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum UniResultError {
	#[error("Incompatible {op:?} operation between {left} and {right}")]
	IncompatibleOperationType {left: UniResultType, op: BinaryOp, right: UniResultType},
	#[error("Incompatible operation for \"{val}\"")]
	IncompatibleUnaryOperationType {val: UniResultType},
	#[error("Using 'into' on incompatible type {0:?}")]
	UsingIntoOnIncompatibleType(UniResult),
	#[error("Unsupported binary operation '{0}'")]
	UnsupportedBinaryOperation(BinaryOp),
	#[error("Unsupported unary operation '{0}'")]
	UnsupportedUnaryOperation(UnaryOp),
	#[error("Cannot convert from {0:?} to UniResult")]
	CannotConvertToUniResult(ASTNode),
	#[error("Cannot convert to {0:?} from UniResult")]
	CannotConvertFromUniResult(UniResult),
}

// todo: сделать типы полноценными объектами с функциями, и тогда выдавать на type_name имя типа

#[derive(Debug, Clone, PartialEq)]
pub enum UniResultType {
	Boolean,
	Number,
	Float,
	Range,
}

impl fmt::Display for UniResultType {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			UniResultType::Boolean 	=> "boolean",
			UniResultType::Number	=> "number",
			UniResultType::Float 	=> "float",
			UniResultType::Range 	=> "range",
		}
		.fmt(f)
	}
}

#[derive(Debug, Clone, PartialEq)]
pub enum UniResult {
	Boolean(bool),
	Number(i32),
	Float(f32),
	Range { start: f32, end: f32, step: f32 },
}

impl TryFrom<Spanned<UniResult>> for Spanned<ASTNode> {
	type Error = Spanned<UniResultError>;

	fn try_from(value: Spanned<UniResult>) -> Result<Self, Self::Error> {
		let span = value.span;

		let result = match value.val {
			UniResult::Boolean(val) =>
				ASTNode::Boolean(val),
			UniResult::Number(val) =>
				ASTNode::Number(val),
			UniResult::Float(val) =>
				ASTNode::Float(val),
			UniResult::Range { .. } =>
				return Err(
					Spanned::from_with_span(
						UniResultError::CannotConvertFromUniResult(value.val),
						span
					)
				),
		};

		Ok(
			Spanned::from_with_span(
				result,
				span
			)
		)
	}
}

impl TryFrom<Spanned<ASTNode>> for Spanned<UniResult> {
	type Error = Spanned<UniResultError>;

	fn try_from(value: Spanned<ASTNode>) -> Result<Self, Self::Error> {
		use UniResult as RawSelf;

		let span = value.span;

		let result = match value.val {
			ASTNode::Boolean(val) => RawSelf::Boolean(val),
			ASTNode::Number(val) => RawSelf::Number(val),
			ASTNode::Float(val) => RawSelf::Float(val),
			v => return Err(
				Spanned::from_with_span(
					UniResultError::CannotConvertToUniResult(v),
					span
				)
			),
		};

		Ok(
			Self::from_with_span(result, span)
		)
	}
}

impl Display for UniResult {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Boolean(x) => write!(f, "{x}"),
			Self::Number(x) => write!(f, "{x}"),
			Self::Float(x) => write!(f, "{x}"),
			Self::Range { start, end, step } => {
				write!(f, "{start}..{end}..{step}")
			},
		}
	}
}

type OpResult = Result<Spanned<UniResult>, Spanned<UniResultError>>;

impl UniResult {
	pub fn result_type(&self) -> UniResultType {
		match self {
			UniResult::Boolean(..) 		=>	UniResultType::Boolean,
			UniResult::Number(..) 		=>	UniResultType::Number,
			UniResult::Float(..) 		=>	UniResultType::Float,
			UniResult::Range {..} 		=>	UniResultType::Range,
		}
	}

	pub fn calc_binary_by_op(left: &Spanned<UniResult>, right: &Spanned<UniResult>, op: &BinaryOp) -> Result<Spanned<UniResult>, Spanned<UniResultError>> {
		match op {
			BinaryOp::Plus 			=> Self::add(left, right),
			BinaryOp::Minus 		=> Self::sub(left, right),
			BinaryOp::Multiply 		=> Self::mul(left, right),
			BinaryOp::Pow 			=> Self::pow(left, right),
			BinaryOp::Divide 		=> Self::div(left, right),
			BinaryOp::Remainder 	=> Self::rem(left, right),
			BinaryOp::Equals 		=> Self::eq(left, right),
			BinaryOp::Great 		=> Self::gt(left, right),
			BinaryOp::Less 			=> Self::lt(left, right),
			BinaryOp::LogicalAnd 	=> Self::log_and(left, right),
			BinaryOp::LogicalOr 	=> Self::log_or(left, right),
			_ => Err(
				Spanned::from_with_span(
					UniResultError::UnsupportedBinaryOperation(op.to_owned()),
					left.span
				)
			),
		}
	}

	pub fn calc_unary_by_op(left: &Spanned<UniResult>, op: &UnaryOp) -> Result<Spanned<UniResult>, Spanned<UniResultError>> {
		match op {
			UnaryOp::Minus 	=> Self::neg(left),
			UnaryOp::Not 	=> Self::not(left),
			_ => Err(
				Spanned::from_with_span(
					UniResultError::UnsupportedUnaryOperation(op.to_owned()),
					left.span
				)
			),
		}
	}

	fn add(left: &Spanned<UniResult>, right: &Spanned<UniResult>) -> OpResult {
		let result = match (&left.val, &right.val) {
			(&Self::Number(a), &Self::Number(b))
				=> Ok(Self::Number(a + b)),
			(&Self::Float(a), &Self::Float(b))
				=> Ok(Self::Float(a + b)),
			(left, right) => Err(
				UniResultError::IncompatibleOperationType {
					left: left.result_type(), op: BinaryOp::Plus, right: right.result_type()
				}
			),
		};

		res_with_pos!(result, left.span)
	}
	fn sub(left: &Spanned<UniResult>, right: &Spanned<UniResult>) -> OpResult {
		let result = match (&left.val, &right.val) {
			(&Self::Number(a), &Self::Number(b))
				=> Ok(Self::Number(a - b)),
			(&Self::Float(a), &Self::Float(b))
				=> Ok(Self::Float(a - b)),
			(left, right) => Err(
				UniResultError::IncompatibleOperationType {
					left: left.result_type(), op: BinaryOp::Minus, right: right.result_type()
				}
			),
		};

		res_with_pos!(result, left.span)
	}
	fn pow(left: &Spanned<UniResult>, right: &Spanned<UniResult>) -> OpResult {
		let result = match (&left.val, &right.val) {
			(&Self::Number(a), &Self::Number(b))
				=> Ok(Self::Number(a.pow(b.cast_unsigned()))),
			(Self::Float(a), &Self::Float(b))
				=> Ok(Self::Float(a.powf(b))),
			(left, right) => Err(
				UniResultError::IncompatibleOperationType {
					left: left.result_type(), op: BinaryOp::Pow, right: right.result_type()
				}
			),
		};

		res_with_pos!(result, left.span)
	}
	fn mul(left: &Spanned<UniResult>, right: &Spanned<UniResult>) -> OpResult {
		let result = match (&left.val, &right.val) {
			(&Self::Number(a), &Self::Number(b))
				=> Ok(Self::Number(a * b)),
			(&Self::Float(a), &Self::Float(b))
				=> Ok(Self::Float(a * b)),
			(left, right) =>
				Err(
					UniResultError::IncompatibleOperationType {
						left: left.result_type(), op: BinaryOp::Multiply, right: right.result_type()
					}
				),
		};

		res_with_pos!(result, left.span)
	}
	fn div(left: &Spanned<UniResult>, right: &Spanned<UniResult>) -> OpResult {
		let result = match (&left.val, &right.val) {
			(&Self::Number(a), &Self::Number(b))
				=> Ok(Self::Number(a / b)),
			(Self::Float(a), &Self::Float(b))
				=> Ok(Self::Float(a / b)),
			(left, right) => Err(
				UniResultError::IncompatibleOperationType {
					left: left.result_type(), op: BinaryOp::Divide, right: right.result_type()
				}
			),
		};

		res_with_pos!(result, left.span)
	}
	fn rem(left: &Spanned<UniResult>, right: &Spanned<UniResult>) -> OpResult {
		let result = match (&left.val, &right.val) {
			(&Self::Number(a), &Self::Number(b))
				=> Ok(Self::Number(a % b)),
			(Self::Float(a), &Self::Float(b))
				=> Ok(Self::Float(a % b)),
			(left, right) => Err(
				UniResultError::IncompatibleOperationType {
					left: left.result_type(), op: BinaryOp::Remainder, right: right.result_type()
				}
			),
		};

		res_with_pos!(result, left.span)
	}

	fn eq(left: &Spanned<UniResult>, right: &Spanned<UniResult>) -> OpResult {
		let result = match (&left.val, &right.val) {
			(Self::Number(a), Self::Number(b))
				=> Ok(Self::Boolean(a == b)),
			(Self::Float(a), &Self::Float(b))
				=> Ok(Self::Boolean((a - b).abs() < f32::EPSILON)),
			(Self::Boolean(a), Self::Boolean(b))
				=> Ok(Self::Boolean(a == b)),
			_ => Err(
				UniResultError::IncompatibleOperationType {
					left: left.result_type(), op: BinaryOp::Equals, right: right.result_type()
				}
			),
		};

		res_with_pos!(result, left.span)
	}
	pub fn lt(left: &Spanned<UniResult>, right: &Spanned<UniResult>) -> OpResult {
		let result = match (&left.val, &right.val) {
			(&Self::Number(a), &Self::Number(b))
				=> Ok(Self::Boolean(a < b)),
			(&Self::Float(a), &Self::Float(b))
				=> Ok(Self::Boolean(a < b)),
			(left, right) => Err(
				UniResultError::IncompatibleOperationType {
					left: left.result_type(), op: BinaryOp::Less, right: right.result_type()
				}
			),
		};

		res_with_pos!(result, left.span)
	}

	pub fn gt(left: &Spanned<UniResult>, right: &Spanned<UniResult>) -> OpResult {
		Self::lt(right, left)
	}

	pub fn log_and(left: &Spanned<UniResult>, right: &Spanned<UniResult>) -> OpResult {
		let result = match (&left.val, &right.val) {
			(&Self::Boolean(a), &Self::Boolean(b))
				=> Ok(Self::Boolean(a && b)),
			(left, right) => Err(
				UniResultError::IncompatibleOperationType {
					left: left.result_type(), op: BinaryOp::LogicalAnd, right: right.result_type()
				}
			),
		};

		res_with_pos!(result, left.span)
	}

	pub fn log_or(left: &Spanned<UniResult>, right: &Spanned<UniResult>) -> OpResult {
		let result = match (&left.val, &right.val) {
			(&Self::Boolean(a), &Self::Boolean(b))
				=> Ok(Self::Boolean(a || b)),
			(left, right) => Err(
				UniResultError::IncompatibleOperationType {
					left: left.result_type(), op: BinaryOp::LogicalOr, right: right.result_type()
				}
			),
		};

		res_with_pos!(result, left.span)
	}

	pub fn neg(value: &Spanned<UniResult>) -> OpResult {
		let result = match &value.val {
			&Self::Number(b)
				=> Ok(Self::Number(-b)),
			&Self::Float(b)
				=> Ok(Self::Float(-b)),
			val => Err(
				UniResultError::IncompatibleUnaryOperationType {
					val: val.result_type()
				}
			),
		};

		res_with_pos!(result, value.span)
	}
	pub fn not(value: &Spanned<UniResult>) -> OpResult {
		let result = match &value.val {
			&Self::Boolean(b)
				=> Ok(Self::Boolean(!b)),
			val => Err(
				UniResultError::IncompatibleUnaryOperationType {
					val: val.result_type()
				}
			),
		};

		res_with_pos!(result, value.span)
	}
}

impl From<bool> for UniResult {
	fn from(value: bool) -> Self {
		Self::Boolean(value)
	}
}

impl From<i32> for UniResult {
	fn from(value: i32) -> Self {
		Self::Number(value)
	}
}

impl From<u32> for UniResult {
	fn from(value: u32) -> Self {
		Self::Number(value.cast_signed())
	}
}

impl From<f32> for UniResult {
	fn from(value: f32) -> Self {
		Self::Float(value)
	}
}

impl TryFrom<UniResult> for bool {
	type Error = UniResultError;

	fn try_from(value: UniResult) -> Result<bool, Self::Error> {
		let UniResult::Boolean(x) = value else {
			return Err(
				UniResultError::UsingIntoOnIncompatibleType(value)
			)
		};

		Ok(x)
	}
}

impl TryFrom<&UniResult> for u32 {
	type Error = UniResultError;

	fn try_from(value: &UniResult) -> Result<u32, Self::Error> {
		let &UniResult::Number(x) = value else {
			return Err(
				UniResultError::UsingIntoOnIncompatibleType(value.clone())
			)
		};

		Ok(x.cast_unsigned())
	}
}

impl TryFrom<&UniResult> for i32 {
	type Error = UniResultError;

	fn try_from(value: &UniResult) -> Result<i32, Self::Error> {
		let &UniResult::Number(x) = value else {
			return Err(
				UniResultError::UsingIntoOnIncompatibleType(value.clone())
			)
		};

		Ok(x)
	}
}

impl TryFrom<&UniResult> for f32 {
	type Error = UniResultError;

	fn try_from(value: &UniResult) -> Result<f32, Self::Error> {
		let &UniResult::Float(x) = value else {
			return Err(
				UniResultError::UsingIntoOnIncompatibleType(value.clone())
			)
		};

		Ok(x)
	}
}

impl TryFrom<Spanned<UniResult>> for bool {
	type Error = Spanned<UniResultError>;

	fn try_from(value: Spanned<UniResult>) -> Result<bool, Self::Error> {
		let UniResult::Boolean(x) = value.val else {
			return Err(
				Spanned::from_with_span(
					UniResultError::UsingIntoOnIncompatibleType(value.val),
					value.span
				)
			)
		};

		Ok(x)
	}
}

impl TryFrom<Spanned<UniResult>> for i32 {
	type Error = Spanned<UniResultError>;

	fn try_from(value: Spanned<UniResult>) -> Result<i32, Self::Error> {
		let UniResult::Number(x) = value.val else {
			return Err(
				Spanned::from_with_span(
					UniResultError::UsingIntoOnIncompatibleType(value.val),
					value.span
				)
			)
		};

		Ok(x)
	}
}

impl TryFrom<Spanned<UniResult>> for f32 {
	type Error = Spanned<UniResultError>;

	fn try_from(value: Spanned<UniResult>) -> Result<f32, Self::Error> {
		let UniResult::Float(x) = value.val else {
			return Err(
				Spanned::from_with_span(
					UniResultError::UsingIntoOnIncompatibleType(value.val),
					value.span
				)
			)
		};

		Ok(x)
	}
}
