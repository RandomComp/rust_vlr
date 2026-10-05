use std::{fmt::Write, collections::VecDeque, mem, num::ParseFloatError};

use crate::lexer::{LexerError, Token, TokenType};

impl From<ParseFloatError> for LexerError {
	fn from(err: ParseFloatError) -> LexerError {
		LexerError::InvalidNumber {column: 0, row: 0, e: err}
	}
}

#[derive(Debug, thiserror::Error)]
pub enum ParserError {
	// #[error("line {row} at {column}: invalid syntax")]
	// InvalidSyntax {row: usize, column: usize},
	#[error("line {row} at {column}: expected {token_type} token, found {found}")]
	ExpectedToken {row: usize, column: usize, token_type: TokenType, found: TokenType},
	#[error("line {row} at {column}: token {token_type} unexpected")]
	UnexpectedToken {row: usize, column: usize, token_type: TokenType},
	#[error("Cannot convert from {0:?} to BinaryOp")]
	CannotConvertToBinaryOp(TokenType),
	#[error("Cannot convert from {0:?} to UnaryOp")]
	CannotConvertToUnaryOp(TokenType),
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
	NotEquals,
	Great,
	Less,
	GreatOrEquals,
	LessOrEquals,
	LogicalAnd,
	LogicalOr,

	Range,
}

impl BinaryOp {
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
			Self::NotEquals |
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
			Self::NotEquals => write!(f, "!="),
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

impl TryFrom<TokenType> for BinaryOp {
	type Error = ParserError;

	fn try_from(value: TokenType) -> Result<Self, Self::Error> {
		let result = match value {
			TokenType::Assignment => Self::Assignment,
			TokenType::PlusAssignment => Self::PlusAssignment,
			TokenType::MinusAssignment => Self::MinusAssignment,
			TokenType::PowAssignment => Self::PowAssignment,
			TokenType::MultiplyAssignment => Self::MultiplyAssignment,
			TokenType::DivideAssignment => Self::DivideAssignment,
			TokenType::RemainderAssignment => Self::RemainderAssignment,

			TokenType::Plus => Self::Plus,
			TokenType::Minus => Self::Minus,
			TokenType::Multiply => Self::Multiply,
			TokenType::Pow => Self::Pow,
			TokenType::Divide => Self::Divide,
			TokenType::Remainder => Self::Remainder,

			TokenType::Equals => Self::Equals,
			TokenType::NotEquals => Self::NotEquals,
			TokenType::GreatOrEquals => Self::GreatOrEquals,
			TokenType::LessOrEquals => Self::LessOrEquals,
			TokenType::Great => Self::Great,
			TokenType::Less => Self::Less,

			TokenType::LogicalAnd => Self::LogicalAnd,
			TokenType::LogicalOr => Self::LogicalOr,

			TokenType::Range => Self::Range,
			v => return Err(ParserError::CannotConvertToBinaryOp(v))
		};

		Ok(result)
	}
}

#[derive(Clone, Debug, PartialEq)]
pub enum UnaryOp {
	Plus,
	Minus,
}

impl std::fmt::Display for UnaryOp {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::Plus => write!(f, "+"),
			Self::Minus => write!(f, "-"),
		}
	}
}

#[derive(Clone, Debug)]
pub enum ASTNodeEnum {
	None,
	Everything,
	Boolean(bool),
	Number(f64),
	String(String),
	Tuple(Vec<ASTNode>),
	Variable(String),
	Block(VecDeque<ASTNode>),
	Break(Box<ASTNode>),
	Return(Box<ASTNode>),
	If {
		condition: Box<ASTNode>,
		block: Box<ASTNode>,
		block_else: Option<Box<ASTNode>>
	},
	While {
		condition: Box<ASTNode>,
		block: Box<ASTNode>,
		block_else: Option<Box<ASTNode>>
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
		block: Box<ASTNode>,
	},
}

#[derive(Clone, Debug)]
pub struct ASTNode {
	pub value: ASTNodeEnum,
	pub row: usize, pub column: usize,
}

impl Default for ASTNode {
	fn default() -> Self {
    	ASTNode::NONE
	}
}

impl From<ASTNodeEnum> for ASTNode {
	fn from(value: ASTNodeEnum) -> Self {
    	ASTNode { value, row: 0, column: 0 }
	}
}

impl ASTNode {
	pub const NONE: Self = Self {value: ASTNodeEnum::None, row: 0, column: 0};

	pub fn format_human_readable(&self, f: &mut String, for_statement: bool, tab_level: usize) -> std::fmt::Result {
		self.value.format_human_readable(f, for_statement, tab_level)
	}
}

impl std::fmt::Display for ASTNodeEnum {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			ASTNodeEnum::None => write!(f, "()"),
			ASTNodeEnum::Everything => write!(f, "..."),
			ASTNodeEnum::Boolean(value) => write!(f, "{value}"),
			ASTNodeEnum::Number(value) => write!(f, "{value}"),
			ASTNodeEnum::String(text) => write!(f, "\"{text}\""),
			ASTNodeEnum::Variable(name) => write!(f, "{name}"),
			ASTNodeEnum::Tuple(values) => {
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
			ASTNodeEnum::Block(values) => {
				write!(f, "{{ ")?;

				for value in values {
					write!(f, "{value}; ")?;
				}

				write!(f, "}}")?;

				Ok(())
			},
			ASTNodeEnum::If { condition, block, block_else: Some(block_else) } => {
				write!(f, "if {condition} {block} else {block_else}")
			},
			ASTNodeEnum::If { condition, block, block_else: None } => {
				write!(f, "if {condition} {block}")
			},
			ASTNodeEnum::Break(value) => {
				write!(f, "break {value}")
			},
			ASTNodeEnum::Return(value) => {
				write!(f, "return {value}")
			},
			ASTNodeEnum::While { condition, block, block_else: Some(block_else) } => {
				write!(f, "while {condition} {block} else {block_else}")
			},
			ASTNodeEnum::While { condition, block, block_else: None } => {
				write!(f, "while {condition} {block}")
			},
			// ASTNodeEnum::Ternary { left, op, center, right } =>
			// 	write!(f, "({} {} {} {} {})", left, op, center, op, right),
			ASTNodeEnum::Binary { left, op, right } =>
				write!(f, "({left} {op} {right})"),
			ASTNodeEnum::Unary {op, value} =>
				write!(f, "{op}({value})"),
			ASTNodeEnum::Function {name, arg} => {
				write!(f, "{name}({arg})")
			},
			ASTNodeEnum::FunctionDefinition { name: Some(name), args, block } => {
				write!(f, "def {name}(")?;

				for (i, arg) in args.iter().enumerate() {
					if i > 0 {
						write!(f, ", ")?;
					}

					write!(f, "{arg}")?;
				}

				write!(f, ") {block}")?;

				Ok(())
			},
			ASTNodeEnum::FunctionDefinition { name: None, args, .. } => {
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

impl ASTNodeEnum {
	pub fn format_human_readable(&self, f: &mut String, for_statement: bool, tab_level: usize) -> std::fmt::Result {
		let tab_level_syms: String = std::iter::repeat_n(' ', 4 * tab_level).collect();

		if !for_statement {
			write!(f, "{tab_level_syms}")?;
		}

		match self {
			ASTNodeEnum::None => write!(f, "()"),
			ASTNodeEnum::Everything => write!(f, "..."),
			ASTNodeEnum::Boolean(value) => write!(f, "{value}"),
			ASTNodeEnum::Number(value) => write!(f, "{value}"),
			ASTNodeEnum::String(text) => write!(f, "\"{text}\""),
			ASTNodeEnum::Variable(name) => write!(f, "{name}"),
			ASTNodeEnum::Tuple(values) => {
				write!(f, "(")?;

				for (i, value) in values.iter().enumerate() {
					if i > 0 {
						write!(f, ", ")?;
					}

					value.format_human_readable(f, false, 0)?;
				}

				write!(f, ")")
			},
			ASTNodeEnum::Block(values) => {
				writeln!(f, "{{")?;

				for value in values {
					value.format_human_readable(f, false, tab_level + 1)?;

					writeln!(f, ";")?;
				}

				write!(f, "{tab_level_syms}}}")
			},
			ASTNodeEnum::If { condition, block, block_else: Some(block_else) } => {
				write!(f, "if ")?;

				condition.format_human_readable(f, false, 0)?;

				write!(f, " ")?;

				block.format_human_readable(f, true, tab_level + 1)?;

				write!(f, " ")?;

				write!(f, "else ")?;

				write!(f, " ")?;

				block_else.format_human_readable(f, true, tab_level + 1)
			},
			ASTNodeEnum::If { condition, block, block_else: None } => {
				write!(f, "if ")?;

				condition.format_human_readable(f, false, 0)?;

				write!(f, " ")?;

				block.format_human_readable(f, true, tab_level)
			},
			ASTNodeEnum::Break(value) => {
				write!(f, "break {value}")
			},
			ASTNodeEnum::Return(value) => {
				write!(f, "return {value}")
			},
			ASTNodeEnum::While { condition, block, block_else: Some(block_else) } => {
				write!(f, "while ")?;

				condition.format_human_readable(f, false, 0)?;

				write!(f, " ")?;

				block.format_human_readable(f, false, tab_level + 1)?;

				write!(f, " ")?;

				write!(f, "else ")?;

				write!(f, " ")?;

				block_else.format_human_readable(f, true, tab_level + 1)
			},
			ASTNodeEnum::While { condition, block, block_else: None } => {
				write!(f, "while ")?;

				condition.format_human_readable(f, false, 0)?;

				write!(f, " ")?;

				block.format_human_readable(f, true, tab_level)
			},
			ASTNodeEnum::Binary { left, op, right } =>
				write!(f, "({left} {op} {right})"),
			ASTNodeEnum::Unary {op, value} =>
				write!(f, "{op}({value})"),
			ASTNodeEnum::Function {name, arg} => {
				write!(f, "{name}({arg})")
			},
			ASTNodeEnum::FunctionDefinition { name: Some(name), args, block } => {
				write!(f, "def {name}(")?;

				for (i, arg) in args.iter().enumerate() {
					if i > 0 {
						write!(f, ", ")?;
					}

					write!(f, "{arg}")?;
				}

				write!(f, ") ")?;

				block.format_human_readable(f, true, tab_level + 1)?;

				Ok(())
			},
			ASTNodeEnum::FunctionDefinition { name: None, args, block } => {
				write!(f, "anonymous def(")?;

				for (i, arg) in args.iter().enumerate() {
					if i > 0 {
						write!(f, ", ")?;
					}

					write!(f, "{arg}")?;
				}

				write!(f, ") ")?;

				block.format_human_readable(f, true, tab_level + 1)
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

	fn skip(&mut self, expected_token_type: &TokenType) {
		while let Some(Token { token_type, .. }) = self.peek(0) && token_type == expected_token_type {
			self.index += 1;
		}
	}

	fn consume_next(&mut self) -> Option<&Token> {
		let result = self.tokens.get(self.index);

		self.index += 1;

		result
	}

	fn consume(&mut self, expected_token_type: &TokenType) -> Result<(), ParserError> {
		match self.peek(0) {
			Some(Token { token_type, .. }) if token_type == expected_token_type => {
				self.consume_next();

				Ok(())
			},
			Some(Token { token_type, row, column }) => {
				Err(ParserError::ExpectedToken { row: *row, column: *column, token_type: expected_token_type.clone(), found: token_type.clone() })
			},
			None => panic!("Outside of array"),
		}
	}

	fn _token_exists_until_token(&self, expected_token_type: &TokenType, until_token_type: &TokenType) -> bool {
		let mut index = 0;

		loop {
			match self.peek(index).map(|v| &v.token_type) {
				Some(token_type) if token_type == until_token_type => break false,
				Some(token_type) if token_type == expected_token_type => break true,
				Some(_) => index += 1,
				None => break false,
			}
		}
	}

	fn parse_block(&mut self) -> Result<Option<ASTNode>, ParserError> {
		self.consume(&TokenType::LBrace)?;

		let result = self.parse_instructions();

		self.consume(&TokenType::RBrace)?;

		result
	}

	pub fn parse_instructions(&mut self) -> Result<Option<ASTNode>, ParserError> {
		let (row, column) = match self.peek(0) {
			Some(Token { row, column, .. }) => (*row, *column),
			None => return Ok(None),
		};

		let mut result: VecDeque<ASTNode> = VecDeque::new();

		loop {
			if let Some(Token { token_type: TokenType::RBrace | TokenType::Eof, .. }) = self.peek(0) {
				break;
			}

			let value = self.parse(TokenType::MAX_PRECEDENCE)?;

			result.push_back(value);

			if let Some(Token { token_type: TokenType::Semicolon, .. }) = self.peek(0) {
				self.skip(&TokenType::Semicolon);
			}
		}

		if result.len() == 1 {
			Ok(result.pop_front())
		} else {
			Ok(Some(
				ASTNode { value: ASTNodeEnum::Block(result), row, column }
			))
		}
	}

	fn parse_tuple(&mut self) -> Result<ASTNode, ParserError> {
		self.consume(&TokenType::LPar)?;

		let mut result: Vec<ASTNode> = Vec::new();

		loop {
			if let Some(&Token { token_type: TokenType::RPar, column, row }) = self.peek(0) {
				self.consume(&TokenType::RPar)?;

				return Ok(ASTNode { value: ASTNodeEnum::Tuple(result), row, column });
			}

			let value = self.parse(TokenType::MAX_PRECEDENCE)?;

			match self.peek(0) {
				Some(Token { token_type: TokenType::Comma, .. }) => {
					result.push(value);

					self.consume_next();
				},
				Some(Token { token_type: TokenType::RPar, column, row }) if result.is_empty() => {
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
		let Some(cur) = self.peek(0) else { return Ok(None) };

		match (cur, self.peek(1)) {
			(&Token {token_type: TokenType::True, column, row}, ..) => {
				self.consume_next();

				Ok(Some(
					ASTNode { value: ASTNodeEnum::Boolean(true), row, column }
				))
			}

			(&Token {token_type: TokenType::False, column, row}, ..) => {
				self.consume_next();

				Ok(Some(
					ASTNode { value: ASTNodeEnum::Boolean(false), row, column }
				))
			}

			(&Token {token_type: TokenType::Return, column, row}, ..) => {
				self.consume_next();

				let value = self.parse(TokenType::MAX_PRECEDENCE)?;

				Ok(Some(
					ASTNode { value: ASTNodeEnum::Return(Box::new(value)), row, column }
				))
			}

			(&Token {token_type: TokenType::Break, column, row}, ..) => {
				self.consume_next();

				let value = self.parse(TokenType::MAX_PRECEDENCE)?;

				Ok(Some(
					ASTNode { value: ASTNodeEnum::Break(Box::new(value)), row, column }
				))
			}

			(&Token {token_type: TokenType::If, column, row}, ..) => {
				self.consume_next();

				let condition = self.parse(TokenType::MAX_PRECEDENCE)?;
				let block = self.parse(TokenType::MAX_PRECEDENCE)?;

				match self.peek(0) {
					Some(&Token { token_type: TokenType::Else, row, column }) => {
						self.consume_next();

						let block_else = self.parse(TokenType::MAX_PRECEDENCE)?;

						Ok(Some(
							ASTNode { value: ASTNodeEnum::If { condition: Box::new(condition), block: Box::new(block), block_else: Some(Box::new(block_else)) }, row, column }
						))
					}

					_ => Ok(Some(
						ASTNode { value: ASTNodeEnum::If { condition: Box::new(condition), block: Box::new(block), block_else: None }, row, column }
					))
				}
			}

			(&Token {token_type: TokenType::While, column, row}, ..) => {
				self.consume_next();

				let condition = self.parse(TokenType::MAX_PRECEDENCE)?;
				let block = self.parse(TokenType::MAX_PRECEDENCE)?;

				match self.peek(0) {
					Some(&Token { token_type: TokenType::Else, row, column }) => {
						self.consume_next();

						let block_else = self.parse(TokenType::MAX_PRECEDENCE)?;

						Ok(Some(
							ASTNode { value: ASTNodeEnum::While { condition: Box::new(condition), block: Box::new(block), block_else: Some(Box::new(block_else)) }, row, column }
						))
					}

					_ => Ok(Some(
						ASTNode { value: ASTNodeEnum::While { condition: Box::new(condition), block: Box::new(block), block_else: None }, row, column }
					))
				}
			}

			(&Token {token_type: TokenType::Def, column, row}, Some(Token { token_type: TokenType::Word(name), .. })) => {
				let name = name.clone();

				self.consume_next();
				self.consume_next();
				self.consume(&TokenType::LPar)?;

				let mut args = Vec::new();

				while let Some(Token { token_type, .. }) = self.peek(0) {
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

				let block = self.parse(TokenType::MAX_PRECEDENCE)?;

				Ok(Some(ASTNode { value:
					ASTNodeEnum::FunctionDefinition { name: Some(name), args, block: Box::new(block) },
					row, column }
				))
			}

			(&Token {token_type: TokenType::Def, column, row}, Some(Token { token_type: TokenType::LPar, row: _, column: _ })) => {
				self.consume_next();
				self.consume(&TokenType::LPar)?;

				let mut args = Vec::new();

				while let Some(Token { token_type, column: _, row: _ }) = self.peek(0) {
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

				let block = self.parse(TokenType::MAX_PRECEDENCE)?;

				Ok(Some(ASTNode { value:
						ASTNodeEnum::FunctionDefinition { name: None, args, block: Box::new(block) },
					row, column }
				))
			}

			(&Token {token_type: TokenType::Everything, column, row}, ..) => {
				self.consume_next();

				Ok(Some(
					ASTNode { value: ASTNodeEnum::Everything, row, column }
				))
			}

			(&Token {token_type: TokenType::Number(value), column, row}, _) => {
				self.consume_next();

				Ok(Some(
					ASTNode { value: ASTNodeEnum::Number(value), row, column }
				))
			}

			(Token {token_type: TokenType::String(value), column, row}, ..) => {
				let (column, row) = (*column, *row);
				let value = value.clone();

				self.consume_next();

				Ok(Some(
					ASTNode { value: ASTNodeEnum::String(value), row, column }
				))
			}

			(&Token {token_type: TokenType::Plus, ..}, ..) => {
				self.consume_next();

				self.parse_unary()
			}

			(&Token {token_type: TokenType::Minus, column, row}, ..) => {
				self.consume_next();

				let Some(value) = self.parse_unary()? else { return Ok(None) };

				Ok(Some(
					ASTNode { value: ASTNodeEnum::Unary { op: UnaryOp::Minus, value: Box::new(value) }, row, column  }
				))
			}

			(&Token {token_type: TokenType::LPar, ..}, ..) => {
				self.parse_tuple().map(Some)
			}

			(&Token {token_type: TokenType::LBrace, ..}, ..) => {
				self.parse_block()
			}

			(Token {token_type: TokenType::Word(word), column, row}, Some(Token { token_type: TokenType::LPar, column: _, row: _ })) => {
				let (row, column) = (*row, *column);

				let word = word.to_owned();

				self.consume_next();

				let value = self.parse_tuple()?;

				Ok(Some(
					ASTNode { value: ASTNodeEnum::Function { name: word, arg: Box::new(value) }, row, column }
				))
			}

			(Token {token_type: TokenType::Word(word), column, row}, ..) => {
				let (column, row) = (*column, *row);
				let word = word.to_owned();

				self.consume_next();

				Ok(Some(
					ASTNode { value: ASTNodeEnum::Variable(word), row, column }
				))
			}

			(Token {token_type, column, row}, ..) =>
				Err(ParserError::UnexpectedToken { row: *row, column: *column, token_type: token_type.clone() }),
		}
	}

	fn parse(&mut self, precedence: usize) -> Result<ASTNode, ParserError> {
		if precedence == 0 {
			return Ok(self.parse_unary()?.expect("Outside of array"));
		}

		let mut first = self.parse(precedence - 1)?;
		let (column, row) = (first.column, first.row);

		let cur = &mut first;

		while let Some(x) = self.peek(0) {
			let Ok(op) = BinaryOp::try_from(x.token_type.clone()) else {
				break;
			};

			if op.precedence() != precedence {
				break;
			}

			let right_precedence = if op.assoc() == OpAssoc::Right {precedence} else {precedence - 1};

			self.consume_next();

			let left = mem::take(cur);
			let right = self.parse(right_precedence)?;

			*cur = ASTNode {
				value: ASTNodeEnum::Binary {
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
