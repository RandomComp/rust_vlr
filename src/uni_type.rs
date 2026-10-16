use std::{fmt::{self, Display, Write}, iter::zip};

use crate::{compiler::parser::{ASTNode, BinaryOp, UnaryOp}, res_with_pos, wrapper::Spanned};

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum UniResultError {
	#[error("Incompatible {op:?} operation between {left} and {right}")]
	IncompatibleOperationType {left: UniResultType, op: BinaryOp, right: UniResultType},
	#[error("Incompatible operation for \"{val}\"")]
	IncompatibleUnaryOperationType {val: UniResultType},
	#[error("Use '{0:?}' on list is ambigous, use all() or any() instead")]
	LogicalOperationOnTupleAmbigous(BinaryOp),
	#[error("Using 'into' on incompatible type {0:?}")]
	UsingIntoOnIncompatibleType(UniResult),
	#[error("Unsupported operation '{0}'")]
	UnsupportedOperation(BinaryOp),
	#[error("Cannot convert from {0:?} to UniResult")]
	CannotConvertToUniResult(ASTNode),
	#[error("Cannot convert to {0:?} from UniResult")]
	CannotConvertFromUniResult(UniResult),
}

// todo: сделать типы полноценными объектами с функциями, и тогда выдавать на type_name имя типа

#[derive(Debug, Clone, PartialEq)]
pub enum UniResultType {
	None,
	Boolean,
	Number,
	Float,
	String,
	List,
	Range,
}

impl fmt::Display for UniResultType {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			UniResultType::None 	=> "none",
			UniResultType::Boolean 	=> "boolean",
			UniResultType::Number	=> "i32 number",
			UniResultType::Float 	=> "float",
			UniResultType::String 	=> "string",
			UniResultType::List 	=> "list",
			UniResultType::Range 	=> "range",
		}
		.fmt(f)
	}
}

#[derive(Default, Debug, Clone, PartialEq)]
pub enum UniResult {
	#[default]
	None,

	Boolean(bool),
	Number(i32),
	Float(f32),
	String(String),
	List(Vec<Spanned<UniResult>>),
	Range { start: f32, end: f32, step: f32 },
}

impl TryFrom<Spanned<UniResult>> for Spanned<ASTNode> {
	type Error = Spanned<UniResultError>;

	fn try_from(value: Spanned<UniResult>) -> Result<Self, Self::Error> {
		let span = value.span;

		let result = match value.val {
			UniResult::None =>
				ASTNode::None,
			UniResult::Boolean(val) =>
				ASTNode::Boolean(val),
			UniResult::Float(val) =>
				ASTNode::Float(val),
			UniResult::String(val) =>
				ASTNode::String(val),
			UniResult::List(values) => {
				let result: Result<Vec<Spanned<ASTNode>>, _> = values.into_iter().map(TryInto::try_into).collect();

				ASTNode::List(result?)
			},
			v => return Err(
				Spanned::from_with_span(
					UniResultError::CannotConvertFromUniResult(v),
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
			ASTNode::None => RawSelf::None,
			ASTNode::Boolean(val) => RawSelf::Boolean(val),
			ASTNode::Float(val) => RawSelf::Float(val),
			ASTNode::String(val) => RawSelf::String(val),
			ASTNode::List(values) => {
				let result: Result<Vec<Self>, _> = values.into_iter().map(TryInto::try_into).collect();

				RawSelf::List(result?)
			},
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
			Self::None => write!(f, "None"),
			Self::Boolean(x) => write!(f, "{x}"),
			Self::Number(x) => write!(f, "{x}"),
			Self::Float(x) => write!(f, "{x}"),
			Self::String(x) => write!(f, "\"{x}\""),
			Self::List(values) => {
				write!(f, "[")?;

				for (i, value) in values.iter().enumerate() {
					if i > 0 {
						write!(f, ", ")?;
					}

					write!(f, "{value}")?;
				}

				write!(f, "]")
			},
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
			UniResult::None 			=>	UniResultType::None,
			UniResult::Boolean(..) 		=>	UniResultType::Boolean,
			UniResult::Number(..) 		=>	UniResultType::Number,
			UniResult::Float(..) 		=>	UniResultType::Float,
			UniResult::String(..) 		=>	UniResultType::String,
			UniResult::List(..) 		=>	UniResultType::List,
			UniResult::Range {..} 		=>	UniResultType::Range,
		}
	}

	pub fn format_to_str(&self, f: &mut String) -> fmt::Result {
		match self {
			Self::String(x) => write!(f, "{x}"),
			Self::List(values) => {
				write!(f, "[")?;

				for (i, value) in values.iter().enumerate() {
					if i > 0 {
						write!(f, ", ")?;
					}

					value.format_to_str(f)?;
				}

				write!(f, "]")
			},
			_ => write!(f, "{self}"),
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
					UniResultError::UnsupportedOperation(op.to_owned()),
					left.span
				)
			),
		}
	}

	pub fn calc_unary_by_op(left: &Spanned<UniResult>, op: &UnaryOp) -> Result<Spanned<UniResult>, Spanned<UniResultError>> {
		match op {
			UnaryOp::Minus 	=> Self::neg(left),
			UnaryOp::Not 	=> Self::not(left),
		}
	}

	fn add(left: &Spanned<UniResult>, right: &Spanned<UniResult>) -> OpResult {
		let result = match (&left.val, &right.val) {
			(&Self::Number(a), &Self::Number(b))
				=> Ok(Self::Number(a + b)),
			(&Self::Float(a), &Self::Float(b))
				=> Ok(Self::Float(a + b)),
			(Self::List(values_a), Self::List(values_b)) => {
				let mut result = Vec::new();

				for (a, b) in zip(values_a, values_b) {
					result.push(Self::add(a, b)?);
				}

				Ok(result.into())
			},
			(Self::List(values), _) => {
				let mut result = Vec::new();

				for value in values {
					result.push(Self::add(value, right)?);
				}

				Ok(result.into())
			},
			(_, Self::List(values)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(Self::add(value, left)?);
				}

				Ok(result.into())
			},
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
			(Self::List(values_a), Self::List(values_b)) => {
				let mut result = Vec::new();

				for (a, b) in zip(values_a, values_b) {
					result.push(Self::add(a, b)?);
				}

				Ok(result.into())
			},
			(Self::List(values), _) => {
				let mut result = Vec::new();

				for value in values {
					result.push(Self::sub(value, right)?);
				}

				Ok(result.into())
			},
			(_, Self::List(values)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(Self::sub(left, value)?);
				}

				Ok(result.into())
			},
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
			(Self::List(values_a), Self::List(values_b)) => {
				let mut result = Vec::new();

				for (a, b) in zip(values_a, values_b) {
					result.push(Self::pow(a, b)?);
				}

				Ok(result.into())
			},
			(Self::List(values), _) => {
				let mut result = Vec::new();

				for value in values {
					result.push(Self::pow(value, right)?);
				}

				Ok(result.into())
			},
			(_, Self::List(values)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(Self::pow(left, value)?);
				}

				Ok(result.into())
			},
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
			(Self::List(values_a), Self::List(values_b)) => {
				let mut result = Vec::new();

				for (a, b) in zip(values_a, values_b) {
					result.push(Self::mul(a, b)?);
				}

				Ok(result.into())
			},
			(Self::List(values), _) => {
				let mut result = Vec::new();

				for value in values {
					result.push(Self::mul(value, right)?);
				}

				Ok(result.into())
			},
			(_, Self::List(values)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(Self::mul(left, value)?);
				}

				Ok(result.into())
			},
			(left, right) => Err(
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
			(Self::List(values_a), Self::List(values_b)) => {
				let mut result = Vec::new();

				for (a, b) in zip(values_a, values_b) {
					result.push(Self::div(a, b)?);
				}

				Ok(result.into())
			},
			(Self::List(values), _) => {
				let mut result = Vec::new();

				for value in values {
					result.push(Self::div(value, right)?);
				}

				Ok(result.into())
			},
			(_, Self::List(values)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(Self::div(left, value)?);
				}

				Ok(result.into())
			},
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
			(Self::List(values_a), Self::List(values_b)) => {
				let mut result = Vec::new();

				for (a, b) in zip(values_a, values_b) {
					result.push(Self::rem(a, b)?);
				}

				Ok(result.into())
			},
			(Self::List(values), _) => {
				let mut result = Vec::new();

				for value in values {
					result.push(Self::rem(value, right)?);
				}

				Ok(result.into())
			},
			(_, Self::List(values)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(Self::rem(left, value)?);
				}

				Ok(result.into())
			},
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
			(Self::None, Self::None)
				=> Ok(Self::Boolean(true)),
			(Self::Number(a), Self::Number(b))
				=> Ok(Self::Boolean(a == b)),
			(Self::Float(a), &Self::Float(b))
				=> Ok(Self::Boolean((a - b).abs() < f32::EPSILON)),
			(Self::String(a), Self::String(b))
				=> Ok(Self::Boolean(a == b)),
			(Self::Boolean(a), Self::Boolean(b))
				=> Ok(Self::Boolean(a == b)),
			(Self::List(values_a), Self::List(values_b)) => {
				let mut result = Vec::new();

				for (a, b) in zip(values_a, values_b) {
					result.push(Self::eq(a, b)?);
				}

				Ok(result.into())
			},
			(Self::List(values), _) => {
				let mut result = Vec::new();

				for value in values {
					result.push(Self::eq(value, right)?);
				}

				Ok(result.into())
			},
			(_, Self::List(values)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(Self::eq(left, value)?);
				}

				Ok(result.into())
			},
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
			(Self::String(a), Self::String(b))
				=> Ok(Self::Boolean(a < b)),
			(Self::List(values_a), Self::List(values_b)) => {
				let mut result = Vec::new();

				for (a, b) in zip(values_a, values_b) {
					result.push(Self::lt(a, b)?);
				}

				Ok(result.into())
			},
			(Self::List(values), _) => {
				let mut result = Vec::new();

				for value in values {
					result.push(Self::lt(value, right)?);
				}

				Ok(result.into())
			},
			(_, Self::List(values)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(Self::lt(left, value)?);
				}

				Ok(result.into())
			},
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
			(Self::List(..) | Self::Boolean(..), Self::List(..) | Self::Boolean(..)) => {
				Err(UniResultError::LogicalOperationOnTupleAmbigous(BinaryOp::LogicalAnd))
			},
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
			(Self::List(..) | Self::Boolean(..), Self::List(..) | Self::Boolean(..)) => {
				Err(UniResultError::LogicalOperationOnTupleAmbigous(BinaryOp::LogicalOr))
			},
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
			Self::List(values) => {
				let mut result = Vec::new();

				for value in values {
					result.push(Self::neg(value)?);
				}

				Ok(result.into())
			},
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
			Self::List(values) => {
				let mut result = Vec::new();

				for value in values {
					result.push(Self::not(value)?);
				}

				Ok(result.into())
			},
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

impl From<String> for UniResult {
	fn from(value: String) -> Self {
		Self::String(value)
	}
}

impl From<Vec<Spanned<UniResult>>> for UniResult {
	fn from(values: Vec<Spanned<UniResult>>) -> Self {
		Self::List(values)
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

impl TryFrom<UniResult> for String {
	type Error = UniResultError;

	fn try_from(value: UniResult) -> Result<String, Self::Error> {
		let UniResult::String(x) = value else {
			return Err(
				UniResultError::UsingIntoOnIncompatibleType(value)
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

impl TryFrom<Spanned<UniResult>> for String {
	type Error = Spanned<UniResultError>;

	fn try_from(value: Spanned<UniResult>) -> Result<String, Self::Error> {
		let UniResult::String(x) = value.val else {
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

impl TryFrom<Spanned<UniResult>> for Vec<Spanned<UniResult>> {
	type Error = Spanned<UniResultError>;

	fn try_from(value: Spanned<UniResult>) -> Result<Vec<Spanned<UniResult>>, Self::Error> {
		let UniResult::List(x) = value.val else {
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
