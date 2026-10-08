 use crate::{res_with_pos, wmatch, wrapper::ASTNode};

use std::{collections::VecDeque, fmt::{self, Write}, mem};

use crate::{lexer::RawToken, val_with_pos, wrapper::{ParserError, Token}};

#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum RawParserError {
	// #[error("line {row} at {column}: invalid syntax")]
	// InvalidSyntax {row: usize, column: usize},
	#[error("expected {kind} token, found {found}")]
	ExpectedToken {kind: RawToken, found: RawToken},
	#[error("expected {} token, found {found}", Self::get_tokens_fmt(types)?)]
	ExpectedTokens {types: &'static [RawToken], found: RawToken},
	#[error("token {token_type} unexpected")]
	UnexpectedToken {token_type: RawToken},
	#[error("Cannot convert from {0:?} to BinaryOp")]
	CannotConvertToBinaryOp(RawToken),
	#[error("Cannot convert from {0:?} to UnaryOp")]
	CannotConvertToUnaryOp(RawToken),
}

impl RawParserError {
	fn get_tokens_fmt(types: &[RawToken]) -> Result<String, fmt::Error> {
		let mut result = String::new();

		for (i, token) in types.iter().enumerate() {
			if i > 0 {
				write!(result, " or ")?;
			}

			write!(result, "'{token}'")?;
		}

		Ok(result)
	}
}

#[derive(Debug, PartialEq)]
pub enum OpAssoc {
	Left,
	Right
}

impl std::fmt::Display for OpAssoc {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Left => write!(f, "left"),
			Self::Right => write!(f, "right")
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
pub enum BinaryOp {
	Plus,
	Minus,
	Pow,
	Multiply,
	Divide,
	Remainder,

	Assignment,
	PlusAssignment,
	MinusAssignment,
	PowAssignment,
	MultiplyAssignment,
	DivideAssignment,
	RemainderAssignment,

	Equals,
	Neq,
	Great,
	Less,
	GreatOrEquals,
	LessOrEquals,
	LogicalAnd,
	LogicalOr,

	Range,
}

impl BinaryOp {
	pub const MAX_PRECEDENCE: usize = 7;

	pub fn precedence(&self) -> usize {
		match self {
			Self::Range => 1,
			Self::Pow => 2,
			Self::Remainder |
			Self::Divide |
			Self::Multiply => 3,
			Self::Plus |
			Self::Minus => 4,

			Self::Equals |
			Self::Neq |
			Self::GreatOrEquals |
			Self::LessOrEquals |
			Self::Great |
			Self::Less => 5,

			Self::LogicalAnd |
			Self::LogicalOr => 6,

			Self::Assignment |
			Self::PlusAssignment |
			Self::MinusAssignment |
			Self::PowAssignment |
			Self::MultiplyAssignment |
			Self::DivideAssignment |
			Self::RemainderAssignment => 7,
		}
	}

	pub fn assoc(&self) -> OpAssoc {
		match self {
			Self::Pow => OpAssoc::Right,
			_ => OpAssoc::Left,
		}
	}
}

impl std::fmt::Display for BinaryOp {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::Assignment => write!(f, "="),
			Self::PlusAssignment => write!(f, "+="),
			Self::MinusAssignment => write!(f, "-="),
			Self::PowAssignment => write!(f, "**="),
			Self::MultiplyAssignment => write!(f, "*="),
			Self::DivideAssignment => write!(f, "/="),
			Self::RemainderAssignment => write!(f, "%="),

			Self::Plus => write!(f, "+"),
			Self::Minus => write!(f, "-"),
			Self::Multiply => write!(f, "*"),
			Self::Pow => write!(f, "**"),
			Self::Divide => write!(f, "/"),
			Self::Remainder => write!(f, "%"),

			Self::Equals => write!(f, "=="),
			Self::Neq => write!(f, "!="),
			Self::GreatOrEquals => write!(f, ">="),
			Self::LessOrEquals => write!(f, "<="),
			Self::Great => write!(f, ">"),
			Self::Less => write!(f, "<"),

			Self::LogicalAnd => write!(f, "&&"),
			Self::LogicalOr => write!(f, "||"),

			Self::Range => write!(f, ".."),
		}
	}
}

macro_rules! bind_enum {
	($value: expr, $token_type: ty, $binary_op: ty, $($t:ident),* $(,)?) => {
		match $value {
			$(
				<$token_type>::$t => Some(<$binary_op>::$t),
			)*
			_ => None
		}
	};
}

impl TryFrom<&Token> for BinaryOp {
	type Error = ParserError;

	fn try_from(value: &Token) -> Result<Self, Self::Error> {
		let result = bind_enum!(value.val, RawToken, Self,
			Assignment,
			PlusAssignment,
			MinusAssignment,
			PowAssignment,
			MultiplyAssignment,
			DivideAssignment,
			RemainderAssignment,
			Plus,
			Minus,
			Multiply,
			Pow,
			Divide,
			Remainder,
			Equals,
			Neq,
			GreatOrEquals,
			LessOrEquals,
			Great,
			Less,
			LogicalAnd,
			LogicalOr,
			Range,
		);

		if let Some(result) = result {
			Ok(result)
		} else {
			Err(val_with_pos!(
				RawParserError::CannotConvertToBinaryOp(value.val.clone()),
				ParserError,
				value.span
			))
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
pub enum UnaryOp {
	Minus,
	Not,
}

impl std::fmt::Display for UnaryOp {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::Minus => write!(f, "-"),
			Self::Not => write!(f, "!"),
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
pub enum RawASTNode {
	None,
	Boolean(bool),
	Float(f64),
	String(String),
	Tuple(Vec<ASTNode>),
	Variable(String),
	Block(VecDeque<ASTNode>),
	Break(Box<ASTNode>),
	Return(Box<ASTNode>),
	If {
		condition: Box<ASTNode>,
		body: Box<ASTNode>,
		else_body: Option<Box<ASTNode>>
	},
	While {
		condition: Box<ASTNode>,
		body: Box<ASTNode>,
		else_body: Option<Box<ASTNode>>
	},
	Binary {
		left: Box<ASTNode>,
		op: BinaryOp,
		right: Box<ASTNode>,
	},
	Unary {
		op: UnaryOp,
		value: Box<ASTNode>,
	},
	Function {
		name: String,
		arg: Box<ASTNode>,
	},
	FunctionDefinition {
		name: Option<String>,
		args: Vec<String>,
		body: Box<ASTNode>,
	},
}

impl Default for RawASTNode {
	fn default() -> Self {
		RawASTNode::None
	}
}

impl std::fmt::Display for RawASTNode {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			RawASTNode::None => write!(f, "()"),
			RawASTNode::Boolean(value) => write!(f, "{value}"),
			RawASTNode::Float(value) => write!(f, "{value}"),
			RawASTNode::String(text) => write!(f, "\"{text}\""),
			RawASTNode::Variable(name) => write!(f, "{name}"),
			RawASTNode::Tuple(values) => {
				write!(f, "(")?;

				for (i, value) in values.iter().enumerate() {
					if i < values.len() - 1 {
						write!(f, "{value}, ")?;
					} else {
						write!(f, "{value}")?;
					}
				}

				write!(f, ")")
			},
			RawASTNode::Block(values) => {
				write!(f, "{{ ")?;

				for value in values {
					write!(f, "{value}; ")?;
				}

				write!(f, "}}")?;

				Ok(())
			},
			RawASTNode::If { condition, body, else_body: Some(else_body) } => {
				write!(f, "if {condition} {body} else {else_body}")
			},
			RawASTNode::If { condition, body, else_body: None } => {
				write!(f, "if {condition} {body}")
			},
			RawASTNode::Break(value) => {
				write!(f, "break {value}")
			},
			RawASTNode::Return(value) => {
				write!(f, "return {value}")
			},
			RawASTNode::While { condition, body, else_body: Some(else_body) } => {
				write!(f, "while {condition} {body} else {else_body}")
			},
			RawASTNode::While { condition, body, else_body: None } => {
				write!(f, "while {condition} {body}")
			},
			// ASTNodeEnum::Ternary { left, op, center, right } =>
			// 	write!(f, "({} {} {} {} {})", left, op, center, op, right),
			RawASTNode::Binary { left, op, right } =>
				write!(f, "({left} {op} {right})"),
			RawASTNode::Unary {op, value} =>
				write!(f, "{op}({value})"),
			RawASTNode::Function {name, arg} => {
				write!(f, "{name}({arg})")
			},
			RawASTNode::FunctionDefinition { name: Some(name), args, body } => {
				write!(f, "def {name}(")?;

				for (i, arg) in args.iter().enumerate() {
					if i > 0 {
						write!(f, ", ")?;
					}

					write!(f, "{arg}")?;
				}

				write!(f, ") {body}")?;

				Ok(())
			},
			RawASTNode::FunctionDefinition { name: None, args, .. } => {
				write!(f, "anonymous def(")?;

				for (i, arg) in args.iter().enumerate() {
					if i > 0 {
						write!(f, ", ")?;
					}

					write!(f, "{arg}")?;
				}

				Ok(())
			},
		}
	}
}

impl RawASTNode {
	pub fn format_human_readable(&self, f: &mut String, for_statement: bool, tab_level: usize) -> std::fmt::Result {
		let tab_level_syms: String = std::iter::repeat_n(' ', 4 * tab_level).collect();

		if !for_statement {
			write!(f, "{tab_level_syms}")?;
		}

		match self {
			RawASTNode::None => write!(f, "()"),
			RawASTNode::Boolean(value) => write!(f, "{value}"),
			RawASTNode::Float(value) => write!(f, "{value}"),
			RawASTNode::String(text) => write!(f, "\"{text}\""),
			RawASTNode::Variable(name) => write!(f, "{name}"),
			RawASTNode::Tuple(values) => {
				write!(f, "(")?;

				for (i, value) in values.iter().enumerate() {
					if i > 0 {
						write!(f, ", ")?;
					}

					value.format_human_readable(f, false, 0)?;
				}

				write!(f, ")")
			},
			RawASTNode::Block(values) => {
				writeln!(f, "{{")?;

				for value in values {
					value.format_human_readable(f, false, tab_level + 1)?;

					writeln!(f, ";")?;
				}

				write!(f, "{tab_level_syms}}}")
			},
			RawASTNode::If { condition, body, else_body: Some(else_body) } => {
				write!(f, "if ")?;

				condition.format_human_readable(f, false, 0)?;

				write!(f, " ")?;

				body.format_human_readable(f, true, tab_level + 1)?;

				write!(f, " ")?;

				write!(f, "else ")?;

				write!(f, " ")?;

				else_body.format_human_readable(f, true, tab_level + 1)
			},
			RawASTNode::If { condition, body, else_body: None } => {
				write!(f, "if ")?;

				condition.format_human_readable(f, false, 0)?;

				write!(f, " ")?;

				body.format_human_readable(f, true, tab_level)
			},
			RawASTNode::Break(value) => {
				write!(f, "break {value}")
			},
			RawASTNode::Return(value) => {
				write!(f, "return {value}")
			},
			RawASTNode::While { condition, body, else_body: Some(else_body) } => {
				write!(f, "while ")?;

				condition.format_human_readable(f, false, 0)?;

				write!(f, " ")?;

				body.format_human_readable(f, false, tab_level + 1)?;

				write!(f, " ")?;

				write!(f, "else ")?;

				write!(f, " ")?;

				else_body.format_human_readable(f, true, tab_level + 1)
			},
			RawASTNode::While { condition, body, else_body: None } => {
				write!(f, "while ")?;

				condition.format_human_readable(f, false, 0)?;

				write!(f, " ")?;

				body.format_human_readable(f, true, tab_level)
			},
			RawASTNode::Binary { left, op, right } =>
				write!(f, "({left} {op} {right})"),
			RawASTNode::Unary {op, value} =>
				write!(f, "{op}({value})"),
			RawASTNode::Function {name, arg} => {
				write!(f, "{name}({arg})")
			},
			RawASTNode::FunctionDefinition { name: Some(name), args, body } => {
				write!(f, "def {name}(")?;

				for (i, arg) in args.iter().enumerate() {
					if i > 0 {
						write!(f, ", ")?;
					}

					write!(f, "{arg}")?;
				}

				write!(f, ") ")?;

				body.format_human_readable(f, true, tab_level)?;

				Ok(())
			},
			RawASTNode::FunctionDefinition { name: None, args, body } => {
				write!(f, "anonymous def(")?;

				for (i, arg) in args.iter().enumerate() {
					if i > 0 {
						write!(f, ", ")?;
					}

					write!(f, "{arg}")?;
				}

				write!(f, ") ")?;

				body.format_human_readable(f, true, tab_level)
			},
		}
	}
}

pub struct Parser {
	tokens: Vec<Token>,
	index: usize,
}

impl Parser {
	pub fn new(tokens: Vec<Token>) -> Self {
		Self {tokens, index: 0}
	}

	fn peek(&self, offset: usize) -> Option<&Token> {
		self.tokens.get(self.index + offset)
	}

	fn skip(&mut self, expected_token_type: &RawToken) {
		while let Some(kind) = self.peek(0) && kind == expected_token_type {
			self.index += 1;
		}
	}

	fn consume_next(&mut self) -> Option<&Token> {
		let result = self.tokens.get(self.index);

		self.index += 1;

		result
	}

	fn consume(&mut self, expected_token_type: &RawToken) -> Result<(), RawParserError> {
		match self.peek(0) {
			Some(kind) if kind == expected_token_type => {
				self.consume_next();

				Ok(())
			},
			Some(kind) => {
				Err(RawParserError::ExpectedToken { kind: expected_token_type.clone(), found: kind.val.clone() })
			},
			None => panic!("Outside of array"),
		}
	}

	fn consumes(&mut self, expected: &'static [RawToken]) -> Result<(), RawParserError> {
		match self.peek(0) {
			Some(kind) if expected.contains(kind) => {
				self.consume_next();

				Ok(())
			},
			Some(kind) => {
				Err(RawParserError::ExpectedTokens { types: expected, found: kind.val.clone() })
			},
			None => panic!("Outside of array"),
		}
	}

	fn _token_exists_until_token(&self, expected_token_type: &RawToken, until_token_type: &RawToken) -> bool {
		let mut index = 0;

		loop {
			match self.peek(index) {
				Some(token_type) if token_type == until_token_type => break false,
				Some(token_type) if token_type == expected_token_type => break true,
				Some(_) => index += 1,
				None => break false,
			}
		}
	}

	fn parse_block(&mut self) -> Result<ASTNode, ParserError> {
		self.consume(&RawToken::LBrace)?;

		self.parse_instructions()
	}

	pub fn parse_instructions(&mut self) -> Result<ASTNode, ParserError> {
		let mut result: VecDeque<ASTNode> = VecDeque::new();

		while let Some(cur) = self.peek(0) && !matches!(&**cur, RawToken::RBrace | RawToken::Eof) {
			self.skip(&RawToken::Semicolon);

			result.push_back(
				self.parse(BinaryOp::MAX_PRECEDENCE)?
			);

			self.consumes(&[RawToken::Semicolon, RawToken::RBrace])?;
		}

		if result.len() > 1 {
			Ok(
				RawASTNode::Block(result).into()
			)
		} else {
			Ok(result.pop_front().unwrap_or(RawASTNode::None.into()))
		}
	}

	fn parse_tuple(&mut self) -> Result<ASTNode, ParserError> {
		self.consume(&RawToken::LPar)?;

		let mut result: Vec<ASTNode> = Vec::new();

		loop {
			if let Some(kind) = self.peek(0) && kind == &RawToken::RPar {
				self.consume(&RawToken::RPar)?;

				return Ok(RawASTNode::Tuple(result).into());
			}

			let value = self.parse(BinaryOp::MAX_PRECEDENCE)?;

			match self.peek(0) {
				Some(wmatch!(RawToken::Comma)) => {
					result.push(value);

					self.consume_next();
				},
				Some(wmatch!(RawToken::RPar)) if result.is_empty() => {
					self.consume(&RawToken::RPar)?;

					return Ok(value)
				},
				Some(_) => {
					result.push(value);
				},
				None => {},
			}
		}
	}

	// fn parse_assignment(&mut self) -> Result<Option<ASTNode>, ParserError> {
	// 	match self.peek(0) {
	// 		Some(Token { token_type: TokenType::TokenWord(name), column, row }) => {
	// 			let (row, column) = (*row, *column);

	// 			let name = name.clone();

	// 			self.consume_next();

	// 			self.consume(TokenType::TokenAssignment)?;

	// 			let value = self.parse(TokenType::MAX_PRECEDENCE)?;

	// 			Ok(Some(
	// 				ASTNode { value: ASTNodeEnum::Assignment {name: name, value: Box::new(value)}, row, column }
	// 			))
	// 		},
	// 		Some(_) => return Ok(Some(self.parse(TokenType::MAX_PRECEDENCE)?)),
	// 		None => return Ok(None),
	// 	}
	// }

	fn parse_unary(&mut self) -> Result<ASTNode, ParserError> {
		let Some(cur) = self.peek(0) else {
			return Ok(RawASTNode::None.into())
		};

		let span = cur.span;

		let result = match (cur, self.peek(1)) {
			(wmatch!(RawToken::True), ..) => {
				self.consume_next();

				Ok(
					RawASTNode::Boolean(true)
				)
			}

			(wmatch!(RawToken::False), ..) => {
				self.consume_next();

				Ok(
					RawASTNode::Boolean(false)
				)
			}

			(wmatch!(RawToken::Return), ..) => {
				self.consume_next();

				let value = self.parse(BinaryOp::MAX_PRECEDENCE)?;

				Ok(
					RawASTNode::Return(Box::new(value))
				)
			}

			(wmatch!(RawToken::Break), ..) => {
				self.consume_next();

				let value = self.parse(BinaryOp::MAX_PRECEDENCE)?;

				Ok(
					RawASTNode::Break(Box::new(value))
				)
			}

			(wmatch!(RawToken::If), ..) => {
				self.consume_next();

				let condition = self.parse(BinaryOp::MAX_PRECEDENCE)?;
				let block = self.parse(BinaryOp::MAX_PRECEDENCE)?;

				match self.peek(0) {
					Some(wmatch!(RawToken::Else)) => {
						self.consume_next();

						let block_else = self.parse(BinaryOp::MAX_PRECEDENCE)?;

						Ok(
							RawASTNode::If {
								condition: Box::new(condition),
								body: Box::new(block),
								else_body: Some(Box::new(block_else))
							}
						)
					}

					_ => Ok(
						RawASTNode::If {
							condition: Box::new(condition),
							body: Box::new(block),
							else_body: None
						}
					)
				}
			}

			(wmatch!(RawToken::While), ..) => {
				self.consume_next();

				let condition = self.parse(BinaryOp::MAX_PRECEDENCE)?;
				let block = self.parse(BinaryOp::MAX_PRECEDENCE)?;

				match self.peek(0) {
					Some(wmatch!(RawToken::Else)) => {
						self.consume_next();

						let block_else = self.parse(BinaryOp::MAX_PRECEDENCE)?;

						Ok(
							RawASTNode::While {
								condition: Box::new(condition),
								body: Box::new(block),
								else_body: Some(Box::new(block_else))
							}
						)
					}

					_ => Ok(
						RawASTNode::While {
							condition: Box::new(condition),
							body: Box::new(block),
							else_body: None
						},
					)
				}
			}

			(wmatch!(RawToken::Def), Some(wmatch!(RawToken::Word(name)))) => {
				let name = name.clone();

				self.consume_next();
				self.consume_next();
				self.consume(&RawToken::LPar)?;

				let mut args = Vec::new();

				while let Some(kind) = self.peek(0) {
					match &kind.val {
						RawToken::Word(arg) => {
							args.push(arg.clone());

							self.consume_next();
						}
						RawToken::Comma => {
							self.consume_next();
						},
						_ => break,
					}
				}

				self.consume(&RawToken::RPar)?;

				let block = self.parse(BinaryOp::MAX_PRECEDENCE)?;

				Ok(
					RawASTNode::FunctionDefinition {
						name: Some(name),
						args,
						body: Box::new(block)
					}
				)
			}

			(wmatch!(RawToken::Def), Some(wmatch!(RawToken::LPar))) => {
				self.consume_next();
				self.consume(&RawToken::LPar)?;

				let mut args = Vec::new();

				while let Some(kind) = self.peek(0) {
					match &**kind {
						RawToken::Word(arg) => {
							args.push(arg.clone());

							self.consume_next();
						}
						RawToken::Comma => {
							self.consume_next();
						},
						_ => break,
					}
				}

				self.consume(&RawToken::RPar)?;

				let block = self.parse(BinaryOp::MAX_PRECEDENCE)?;

				Ok(
					RawASTNode::FunctionDefinition {
						name: None,
						args,
						body: Box::new(block)
					},
				)
			}

			// (&Token {token_type: TokenType::Everything, column, row}, ..) => {
			// 	self.consume_next();

			// 	Ok(Some(
			// 		ASTNode { value: ASTNodeEnum::Everything, row, column }
			// 	))
			// }

			(&wmatch!(RawToken::Float(value)), _) => {
				self.consume_next();

				Ok(
					RawASTNode::Float(value),
				)
			}

			(wmatch!(RawToken::String(value)), ..) => {
				let value = value.clone();

				self.consume_next();

				Ok(
					RawASTNode::String(value)
				)
			}

			// TODO: сделать генерацию парсинга унарных операторов

			(wmatch!(RawToken::Plus), ..) => {
				self.consume_next();

				return self.parse_unary()
			}

			(wmatch!(RawToken::Minus), ..) => {
				self.consume_next();

				let value = self.parse_unary()?;

				Ok(
					RawASTNode::Unary {
						op: UnaryOp::Minus,
						value: Box::new(value)
					}
				)
			}

			(wmatch!(RawToken::Exclamation), ..) => {
				self.consume_next();

				let value = self.parse_unary()?;

				Ok(
					RawASTNode::Unary {
						op: UnaryOp::Not,
						value: Box::new(value)
					},
				)
			}

			(wmatch!(RawToken::LPar), ..) => {
				self.parse_tuple().map(|v| v.val)
			}

			(wmatch!(RawToken::LBrace), ..) => {
				self.parse_block().map(|v| v.val)
			}

			(wmatch!(RawToken::Word(word)), Some(wmatch!(RawToken::LPar))) => {
				let word = word.to_owned();

				self.consume_next();

				let value = self.parse_tuple()?;

				Ok(
					RawASTNode::Function {
						name: word,
						arg: Box::new(value)
					}
				)
			}

			(wmatch!(RawToken::Word(word)), ..) => {
				let word = word.to_owned();

				self.consume_next();

				Ok(
					RawASTNode::Variable(word),
				)
			}

			(kind, ..) => {
				let value: Result<RawASTNode, ParserError> = Err(RawParserError::UnexpectedToken { token_type: kind.val.clone() }.into());

				value
			},
		};

		res_with_pos!(result, ASTNode, ParserError, span)
	}

	fn parse(&mut self, precedence: usize) -> Result<ASTNode, ParserError> {
		if precedence == 0 {
			return self.parse_unary()
		}

		let mut first = self.parse(precedence - 1)?;
		let span = first.span;

		let cur = &mut first;

		while let Some(x) = self.peek(0) &&
			let Ok(op) = BinaryOp::try_from(x) &&
			op.precedence() == precedence {
			let right_precedence = if op.assoc() == OpAssoc::Right {precedence} else {precedence - 1};

			self.consume_next();

			let left = mem::take(cur);
			let right = self.parse(right_precedence)?;

			*cur = ASTNode::from(
				RawASTNode::Binary {
					left: Box::new(left),
					op,
					right: Box::new(right)
				}
			).with_pos(span);
		}

		Ok(first)
	}
}
