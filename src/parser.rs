use std::{collections::VecDeque, fmt::{self, Write}, mem};

use crate::{lexer::RawToken, wrapper::{ParserError, Token}};

#[derive(Debug, thiserror::Error)]
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
		let result = bind_enum!(&**value, RawToken, Self,
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

		} else {
			Err(RawParserError::CannotConvertToBinaryOp(value.into().clone()).into().with_pos())
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
pub enum ASTNode {
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

impl Default for ASTNode {
	fn default() -> Self {
    	ASTNode::NONE
	}
}

impl From<ASTNode> for ASTNode {
	fn from(value: ASTNode) -> Self {
    	ASTNode { value, row: 0, column: 0 }
	}
}

impl ASTNode {
	pub const NONE: Self = Self {value: ASTNode::None, row: 0, column: 0};

	pub fn format_human_readable(&self, f: &mut String, for_statement: bool, tab_level: usize) -> std::fmt::Result {
		self.value.format_human_readable(f, for_statement, tab_level)
	}
}

impl std::fmt::Display for ASTNode {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			ASTNode::None => write!(f, "()"),
			ASTNode::Boolean(value) => write!(f, "{value}"),
			ASTNode::Float(value) => write!(f, "{value}"),
			ASTNode::String(text) => write!(f, "\"{text}\""),
			ASTNode::Variable(name) => write!(f, "{name}"),
			ASTNode::Tuple(values) => {
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
			ASTNode::Block(values) => {
				write!(f, "{{ ")?;

				for value in values {
					write!(f, "{value}; ")?;
				}

				write!(f, "}}")?;

				Ok(())
			},
			ASTNode::If { condition, body, else_body: Some(else_body) } => {
				write!(f, "if {condition} {body} else {else_body}")
			},
			ASTNode::If { condition, body, else_body: None } => {
				write!(f, "if {condition} {body}")
			},
			ASTNode::Break(value) => {
				write!(f, "break {value}")
			},
			ASTNode::Return(value) => {
				write!(f, "return {value}")
			},
			ASTNode::While { condition, body, else_body: Some(else_body) } => {
				write!(f, "while {condition} {body} else {else_body}")
			},
			ASTNode::While { condition, body, else_body: None } => {
				write!(f, "while {condition} {body}")
			},
			// ASTNodeEnum::Ternary { left, op, center, right } =>
			// 	write!(f, "({} {} {} {} {})", left, op, center, op, right),
			ASTNode::Binary { left, op, right } =>
				write!(f, "({left} {op} {right})"),
			ASTNode::Unary {op, value} =>
				write!(f, "{op}({value})"),
			ASTNode::Function {name, arg} => {
				write!(f, "{name}({arg})")
			},
			ASTNode::FunctionDefinition { name: Some(name), args, body } => {
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
			ASTNode::FunctionDefinition { name: None, args, .. } => {
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

impl ASTNode {
	pub fn format_human_readable(&self, f: &mut String, for_statement: bool, tab_level: usize) -> std::fmt::Result {
		let tab_level_syms: String = std::iter::repeat_n(' ', 4 * tab_level).collect();

		if !for_statement {
			write!(f, "{tab_level_syms}")?;
		}

		match self {
			ASTNode::None => write!(f, "()"),
			ASTNode::Boolean(value) => write!(f, "{value}"),
			ASTNode::Float(value) => write!(f, "{value}"),
			ASTNode::String(text) => write!(f, "\"{text}\""),
			ASTNode::Variable(name) => write!(f, "{name}"),
			ASTNode::Tuple(values) => {
				write!(f, "(")?;

				for (i, value) in values.iter().enumerate() {
					if i > 0 {
						write!(f, ", ")?;
					}

					value.format_human_readable(f, false, 0)?;
				}

				write!(f, ")")
			},
			ASTNode::Block(values) => {
				writeln!(f, "{{")?;

				for value in values {
					value.format_human_readable(f, false, tab_level + 1)?;

					writeln!(f, ";")?;
				}

				write!(f, "{tab_level_syms}}}")
			},
			ASTNode::If { condition, body, else_body: Some(else_body) } => {
				write!(f, "if ")?;

				condition.format_human_readable(f, false, 0)?;

				write!(f, " ")?;

				body.format_human_readable(f, true, tab_level + 1)?;

				write!(f, " ")?;

				write!(f, "else ")?;

				write!(f, " ")?;

				else_body.format_human_readable(f, true, tab_level + 1)
			},
			ASTNode::If { condition, body, else_body: None } => {
				write!(f, "if ")?;

				condition.format_human_readable(f, false, 0)?;

				write!(f, " ")?;

				body.format_human_readable(f, true, tab_level)
			},
			ASTNode::Break(value) => {
				write!(f, "break {value}")
			},
			ASTNode::Return(value) => {
				write!(f, "return {value}")
			},
			ASTNode::While { condition, body, else_body: Some(else_body) } => {
				write!(f, "while ")?;

				condition.format_human_readable(f, false, 0)?;

				write!(f, " ")?;

				body.format_human_readable(f, false, tab_level + 1)?;

				write!(f, " ")?;

				write!(f, "else ")?;

				write!(f, " ")?;

				else_body.format_human_readable(f, true, tab_level + 1)
			},
			ASTNode::While { condition, body, else_body: None } => {
				write!(f, "while ")?;

				condition.format_human_readable(f, false, 0)?;

				write!(f, " ")?;

				body.format_human_readable(f, true, tab_level)
			},
			ASTNode::Binary { left, op, right } =>
				write!(f, "({left} {op} {right})"),
			ASTNode::Unary {op, value} =>
				write!(f, "{op}({value})"),
			ASTNode::Function {name, arg} => {
				write!(f, "{name}({arg})")
			},
			ASTNode::FunctionDefinition { name: Some(name), args, body } => {
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
			ASTNode::FunctionDefinition { name: None, args, body } => {
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

impl std::fmt::Display for ASTNode {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{}", self.value)
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
		while let Some(RawToken { kind: token_type, .. }) = self.peek(0) && token_type == expected_token_type {
			self.index += 1;
		}
	}

	fn consume_next(&mut self) -> Option<&RawToken> {
		let result = self.tokens.get(self.index);

		self.index += 1;

		result
	}

	fn consume(&mut self, expected_token_type: &TokenType) -> Result<(), ParserError> {
		match self.peek(0) {
			Some(RawToken { kind: token_type, .. }) if token_type == expected_token_type => {
				self.consume_next();

				Ok(())
			},
			Some(RawToken { kind, row, column }) => {
				Err(ParserError::ExpectedToken { row: *row, column: *column, kind: expected_token_type.clone(), found: kind.clone() })
			},
			None => panic!("Outside of array"),
		}
	}

	fn consumes(&mut self, expected: &'static [TokenType]) -> Result<(), ParserError> {
		match self.peek(0) {
			Some(RawToken { kind, .. }) if expected.contains(kind) => {
				self.consume_next();

				Ok(())
			},
			Some(RawToken { kind, row, column }) => {
				Err(ParserError::ExpectedTokens { row: *row, column: *column, types: expected, found: kind.clone() })
			},
			None => panic!("Outside of array"),
		}
	}

	fn _token_exists_until_token(&self, expected_token_type: &TokenType, until_token_type: &TokenType) -> bool {
		let mut index = 0;

		loop {
			match self.peek(index).map(|v| &v.kind) {
				Some(token_type) if token_type == until_token_type => break false,
				Some(token_type) if token_type == expected_token_type => break true,
				Some(_) => index += 1,
				None => break false,
			}
		}
	}

	fn parse_block(&mut self) -> Result<Option<ASTNode>, ParserError> {
		self.consume(&TokenType::LBrace)?;

		self.parse_instructions()
	}

	pub fn parse_instructions(&mut self) -> Result<Option<ASTNode>, ParserError> {
		let Some((row, column)) = self.peek(0).map(|v| (v.row, v.column)) else {
			return Ok(None)
		};

		let mut result: VecDeque<ASTNode> = VecDeque::new();

		while let Some(cur) = self.peek(0) && !matches!(cur.kind, TokenType::RBrace | TokenType::Eof) {
			self.skip(&TokenType::Semicolon);

			result.push_back(
				self.parse(BinaryOp::MAX_PRECEDENCE)?
			);

			self.consumes(&[TokenType::Semicolon, TokenType::RBrace])?;
		}

		if result.len() > 1 {
			Ok(Some(
				ASTNode { value: ASTNode::Block(result), row, column }
			))
		} else {
			Ok(result.pop_front())
		}
	}

	fn parse_tuple(&mut self) -> Result<ASTNode, ParserError> {
		self.consume(&TokenType::LPar)?;

		let mut result: Vec<ASTNode> = Vec::new();

		loop {
			if let Some(&RawToken { kind: TokenType::RPar, column, row }) = self.peek(0) {
				self.consume(&TokenType::RPar)?;

				return Ok(ASTNode { value: ASTNode::Tuple(result), row, column });
			}

			let value = self.parse(BinaryOp::MAX_PRECEDENCE)?;

			match self.peek(0) {
				Some(RawToken { kind: TokenType::Comma, .. }) => {
					result.push(value);

					self.consume_next();
				},
				Some(RawToken { kind: TokenType::RPar, column, row }) if result.is_empty() => {
					self.consume(&TokenType::RPar)?;

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

	fn parse_unary(&mut self) -> Result<Option<ASTNode>, ParserError> {
		let Some(cur) = self.peek(0) else {
			return Ok(None)
		};

		match (cur, self.peek(1)) {
			(&RawToken {kind: TokenType::True, column, row}, ..) => {
				self.consume_next();

				Ok(Some(
					ASTNode { value: ASTNode::Boolean(true), row, column }
				))
			}

			(&RawToken {kind: TokenType::False, column, row}, ..) => {
				self.consume_next();

				Ok(Some(
					ASTNode { value: ASTNode::Boolean(false), row, column }
				))
			}

			(&RawToken {kind: TokenType::Return, column, row}, ..) => {
				self.consume_next();

				let value = self.parse(BinaryOp::MAX_PRECEDENCE)?;

				Ok(Some(
					ASTNode { value: ASTNode::Return(Box::new(value)), row, column }
				))
			}

			(&RawToken {kind: TokenType::Break, column, row}, ..) => {
				self.consume_next();

				let value = self.parse(BinaryOp::MAX_PRECEDENCE)?;

				Ok(Some(
					ASTNode { value: ASTNode::Break(Box::new(value)), row, column }
				))
			}

			(&RawToken {kind: TokenType::If, column, row}, ..) => {
				self.consume_next();

				let condition = self.parse(BinaryOp::MAX_PRECEDENCE)?;
				let block = self.parse(BinaryOp::MAX_PRECEDENCE)?;

				match self.peek(0) {
					Some(&RawToken { kind: RawToken::Else, row, column }) => {
						self.consume_next();

						let block_else = self.parse(BinaryOp::MAX_PRECEDENCE)?;

						Ok(Some(
							ASTNode { value: ASTNode::If { condition: Box::new(condition), body: Box::new(block), else_body: Some(Box::new(block_else)) }, row, column }
						))
					}

					_ => Ok(Some(
						ASTNode { value: ASTNode::If { condition: Box::new(condition), body: Box::new(block), else_body: None }, row, column }
					))
				}
			}

			(&RawToken {kind: RawToken::While, column, row}, ..) => {
				self.consume_next();

				let condition = self.parse(BinaryOp::MAX_PRECEDENCE)?;
				let block = self.parse(BinaryOp::MAX_PRECEDENCE)?;

				match self.peek(0) {
					Some(&RawToken { kind: TokenType::Else, row, column }) => {
						self.consume_next();

						let block_else = self.parse(BinaryOp::MAX_PRECEDENCE)?;

						Ok(Some(
							ASTNode { value: ASTNode::While { condition: Box::new(condition), body: Box::new(block), else_body: Some(Box::new(block_else)) }, row, column }
						))
					}

					_ => Ok(Some(
						ASTNode { value: ASTNode::While { condition: Box::new(condition), body: Box::new(block), else_body: None }, row, column }
					))
				}
			}

			(&RawToken {kind: TokenType::Def, column, row}, Some(RawToken { kind: TokenType::Word(name), .. })) => {
				let name = name.clone();

				self.consume_next();
				self.consume_next();
				self.consume(&TokenType::LPar)?;

				let mut args = Vec::new();

				while let Some(RawToken { kind: token_type, .. }) = self.peek(0) {
					match token_type {
						TokenType::Word(arg) => {
							args.push(arg.clone());

							self.consume_next();
						}
						TokenType::Comma => {
							self.consume_next();
						},
						_ => break,
					}
				}

				self.consume(&TokenType::RPar)?;

				let block = self.parse(BinaryOp::MAX_PRECEDENCE)?;

				Ok(Some(ASTNode { value:
					ASTNode::FunctionDefinition { name: Some(name), args, body: Box::new(block) },
					row, column }
				))
			}

			(&RawToken {kind: TokenType::Def, column, row}, Some(RawToken { kind: TokenType::LPar, row: _, column: _ })) => {
				self.consume_next();
				self.consume(&TokenType::LPar)?;

				let mut args = Vec::new();

				while let Some(RawToken { kind: token_type, column: _, row: _ }) = self.peek(0) {
					match token_type {
						TokenType::Word(arg) => {
							args.push(arg.clone());

							self.consume_next();
						}
						TokenType::Comma => {
							self.consume_next();
						},
						_ => break,
					}
				}

				self.consume(&TokenType::RPar)?;

				let block = self.parse(BinaryOp::MAX_PRECEDENCE)?;

				Ok(Some(ASTNode { value:
						ASTNode::FunctionDefinition { name: None, args, body: Box::new(block) },
					row, column }
				))
			}

			// (&Token {token_type: TokenType::Everything, column, row}, ..) => {
			// 	self.consume_next();

			// 	Ok(Some(
			// 		ASTNode { value: ASTNodeEnum::Everything, row, column }
			// 	))
			// }

			(&RawToken {kind: TokenType::Float(value), column, row}, _) => {
				self.consume_next();

				Ok(Some(
					ASTNode { value: ASTNode::Float(value), row, column }
				))
			}

			(RawToken {kind: TokenType::String(value), column, row}, ..) => {
				let (column, row) = (*column, *row);
				let value = value.clone();

				self.consume_next();

				Ok(Some(
					ASTNode { value: ASTNode::String(value), row, column }
				))
			}

			// TODO: сделать генерацию парсинга унарных операторов

			(&RawToken {kind: TokenType::Plus, ..}, ..) => {
				self.consume_next();

				self.parse_unary()
			}

			(&RawToken {kind: TokenType::Minus, column, row}, ..) => {
				self.consume_next();

				let Some(value) = self.parse_unary()? else { return Ok(None) };

				Ok(Some(
					ASTNode { value: ASTNode::Unary { op: UnaryOp::Minus, value: Box::new(value) }, row, column  }
				))
			}

			(&RawToken {kind: TokenType::Exclamation, column, row}, ..) => {
				self.consume_next();

				let Some(value) = self.parse_unary()? else { return Ok(None) };

				Ok(Some(
					ASTNode { value: ASTNode::Unary { op: UnaryOp::Not, value: Box::new(value) }, row, column  }
				))
			}

			(&RawToken {kind: TokenType::LPar, ..}, ..) => {
				self.parse_tuple().map(Some)
			}

			(&RawToken {kind: TokenType::LBrace, ..}, ..) => {
				self.parse_block()
			}

			(RawToken {kind: TokenType::Word(word), column, row}, Some(RawToken { kind: TokenType::LPar, column: _, row: _ })) => {
				let (row, column) = (*row, *column);

				let word = word.to_owned();

				self.consume_next();

				let value = self.parse_tuple()?;

				Ok(Some(
					ASTNode { value: ASTNode::Function { name: word, arg: Box::new(value) }, row, column }
				))
			}

			(RawToken {kind: TokenType::Word(word), column, row}, ..) => {
				let (column, row) = (*column, *row);
				let word = word.to_owned();

				self.consume_next();

				Ok(Some(
					ASTNode { value: ASTNode::Variable(word), row, column }
				))
			}

			(RawToken {kind: token_type, column, row}, ..) =>
				Err(ParserError::UnexpectedToken { row: *row, column: *column, token_type: token_type.clone() }),
		}
	}

	fn parse(&mut self, precedence: usize) -> Result<ASTNode, ParserError> {
		if precedence == 0 {
			if let Some(result) = self.parse_unary()? {
				return Ok(result)
			}

			return Ok(ASTNode::NONE)
		}

		let mut first = self.parse(precedence - 1)?;
		let (column, row) = (first.column, first.row);

		let cur = &mut first;

		while let Some(x) = self.peek(0) &&
			let Ok(op) = BinaryOp::try_from(&x.kind) &&
			op.precedence() == precedence {
			let right_precedence = if op.assoc() == OpAssoc::Right {precedence} else {precedence - 1};

			self.consume_next();

			let left = mem::take(cur);
			let right = self.parse(right_precedence)?;

			*cur = ASTNode {
				value: ASTNode::Binary {
					left: Box::new(left),
					op,
					right: Box::new(right)
				},
				row, column
			};
		}

		Ok(first)
	}
}
