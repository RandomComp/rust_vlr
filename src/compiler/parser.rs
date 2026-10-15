mod formatter;

use crate::{compiler::lexer::TokenKind, wmatch, wrapper::Spanned};

use std::{fmt::{self, Write}, mem};

use crate::{compiler::lexer::Token};

#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum ParserError {
	// #[error("line {row} at {column}: invalid syntax")]
	// InvalidSyntax {row: usize, column: usize},
	#[error("expected '{kind}' token, found {found}")]
	ExpectedToken {kind: TokenKind, found: Token},

	#[error("expected expression, found {found}")]
	ExpectedExpression {found: Token},

	#[error("expected {}, found {found}", Self::get_tokens_fmt(types)?)]
	ExpectedTokens {types: &'static [TokenKind], found: Token},

	#[error("Cannot convert from {0:?} to BinaryOp")]
	CannotConvertToBinaryOp(Token),

	#[error("Cannot convert from {0:?} to UnaryOp")]
	CannotConvertToUnaryOp(Token),
}

impl ParserError {
	fn get_tokens_fmt(types: &[TokenKind]) -> Result<String, fmt::Error> {
		let mut result = types.iter().take(types.len() - 1).map(ToString::to_string).collect::<Vec<String>>().join(", ");

		if let Some(token) = types.iter().last() {
			write!(result, " or {token}")?;
		}

		Ok(result)
	}
}

#[derive(Debug, PartialEq)]
pub enum OpAssoc {
	Left,
	Right
}

impl fmt::Display for OpAssoc {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Left => "left",
			Self::Right => "right"
		}.fmt(f)
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

impl fmt::Display for BinaryOp {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Self::Assignment => "=",
			Self::PlusAssignment => "+=",
			Self::MinusAssignment => "-=",
			Self::PowAssignment => "**=",
			Self::MultiplyAssignment => "*=",
			Self::DivideAssignment => "/=",
			Self::RemainderAssignment => "%=",

			Self::Plus => "+",
			Self::Minus => "-",
			Self::Multiply => "*",
			Self::Pow => "**",
			Self::Divide => "/",
			Self::Remainder => "%",

			Self::Equals => "==",
			Self::Neq => "!=",
			Self::GreatOrEquals => ">=",
			Self::LessOrEquals => "<=",
			Self::Great => ">",
			Self::Less => "<",

			Self::LogicalAnd => "&&",
			Self::LogicalOr => "||",

			Self::Range => "..",
		}.fmt(f)
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

impl TryFrom<&Spanned<Token>> for BinaryOp {
	type Error = Spanned<ParserError>;

	fn try_from(value: &Spanned<Token>) -> Result<Self, Self::Error> {
		let result = bind_enum!(value.val, Token, Self,
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

		let Some(result) = result else {
			return Err(
				Spanned::from_with_span(
					ParserError::CannotConvertToBinaryOp(value.val.clone()),
					value.span
				)
			)
		};

		Ok(result)
	}
}

#[derive(Clone, Debug, PartialEq)]
pub enum UnaryOp {
	Minus,
	Not,
}

impl fmt::Display for UnaryOp {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Self::Minus => write!(f, "-"),
			Self::Not => write!(f, "!"),
		}
	}
}

#[derive(Clone, Default, Debug, PartialEq)]
pub enum ASTNode {
	#[default]
	None,

	Boolean(bool),
	Number(i32),
	Float(f32),
	String(String),
	List(Vec<Spanned<ASTNode>>),
	Variable(String),
	Block(Vec<Spanned<ASTNode>>),
	Break(Box<Spanned<ASTNode>>),
	Return(Box<Spanned<ASTNode>>),
	If {
		condition: Box<Spanned<ASTNode>>,
		body: Box<Spanned<ASTNode>>,
		else_body: Option<Box<Spanned<ASTNode>>>
	},
	While {
		condition: Box<Spanned<ASTNode>>,
		body: Box<Spanned<ASTNode>>,
		else_body: Option<Box<Spanned<ASTNode>>>
	},
	Binary {
		left: Box<Spanned<ASTNode>>,
		op: Spanned<BinaryOp>,
		right: Box<Spanned<ASTNode>>,
	},
	Unary {
		op: UnaryOp,
		value: Box<Spanned<ASTNode>>,
	},
	Function {
		name: String,
		args: Vec<Spanned<ASTNode>>,
	},
	FunctionDefinition {
		name: Option<Spanned<String>>,
		args: Vec<Spanned<String>>,
		body: Box<Spanned<ASTNode>>,
		is_global: bool,
	},
}

pub struct Parser {
	tokens: Vec<Spanned<Token>>,
	index: usize,
}

impl Parser {
	pub fn new(tokens: Vec<Spanned<Token>>) -> Self {
		Self {tokens, index: 0}
	}

	fn peek(&self, offset: isize) -> Option<&Spanned<Token>> {
		self.tokens.get((self.index as isize + offset) as usize)
	}

	fn skip(&mut self, expected_token_type: TokenKind) {
		while let Some(kind) = self.peek(0) && expected_token_type == (&kind.val).into() {
			self.index += 1;
		}
	}

	fn consume_next(&mut self) -> Option<&Spanned<Token>> {
		let result = self.tokens.get(self.index);

		self.index += 1;

		result
	}

	fn expected(&mut self, expected: TokenKind) -> Spanned<ParserError> {
		match self.peek(0) {
			Some(kind) if expected == (&kind.val).into() => {
				unreachable!("Called expected on a expected token");
			},
			Some(kind) => {
				Spanned::from_with_span(
					ParserError::ExpectedToken {
						kind: expected,
						found: kind.val.clone()
					},
					kind.span
				)
			},
			None => unreachable!("EOF missing or logical error"),
		}
	}

	fn consume(&mut self, expected_token_type: TokenKind) -> Result<(), Spanned<ParserError>> {
		match self.peek(0) {
			Some(kind) if expected_token_type == (&kind.val).into() => {
				self.consume_next();

				Ok(())
			},
			_ => Err(self.expected(expected_token_type)),
		}
	}

	fn consumes(&mut self, expected: &'static [TokenKind]) -> Result<(), ParserError> {
		match self.peek(0) {
			Some(kind) if expected.contains(&(&kind.val).into()) => {
				self.consume_next();

				Ok(())
			},
			Some(kind) => {
				Err(ParserError::ExpectedTokens { types: expected, found: kind.val.clone() })
			},
			None => unreachable!("EOF missing or logical error"),
		}
	}

	fn _token_exists_until_token(&self, expected_token_type: &Token, until_token_type: &Token) -> bool {
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

	fn parse_block(&mut self) -> Result<Spanned<ASTNode>, Spanned<ParserError>> {
		self.consume(TokenKind::LBrace)?;

		let result = self.parse_instructions()?;

		self.consume(TokenKind::RBrace)?;

		Ok(result)
	}

	pub fn parse_instructions(&mut self) -> Result<Spanned<ASTNode>, Spanned<ParserError>> {
		let Some(cur) = self.peek(0) else {
			return Ok(
				Spanned::from_with_span(ASTNode::None, 0) // fix: span
			)
		};

		let span = cur.span;

		let mut result = Vec::new();

		let mut i = 0;

		loop {
			if i > 0 {
				let Some(kind) = self.peek(-1) else {
					unreachable!("peek error or EOF ignoring");
				};

				if *kind != Token::RBrace {
					self.consume(TokenKind::Semicolon)?;
				}

				self.skip(TokenKind::Semicolon);
			}

			if let Some(kind) = self.peek(0) && (*kind == Token::Eof || *kind == Token::RBrace) {
				result.push(Spanned { val: ASTNode::None, span: kind.span });

				break
			}

			let value = self.parse(BinaryOp::MAX_PRECEDENCE)?;

			result.push(value);

			if let Some(kind) = self.peek(0) && (*kind == Token::Eof || *kind == Token::RBrace) {
				break
			}

			i += 1;
		}

		// if result.len() > 1 {
			Ok(
				Spanned::from_with_span(
					ASTNode::Block(result),
					span
				)
			)
		// } else {
		// 	Ok(result.pop().unwrap_or(ASTNode::None))
		// }
	}

	fn parse_elements(&mut self, end_sym: &Token) -> Result<Vec<Spanned<ASTNode>>, Spanned<ParserError>> {
		let mut result = Vec::new();

		let mut i = 0;

		while let Some(kind) = self.peek(0) && kind != end_sym {
			if i > 0 {
				self.consume(TokenKind::Comma)?;
			}

			let value = self.parse(BinaryOp::MAX_PRECEDENCE)?;

			result.push(value);

			i += 1;
		}

		Ok(result)
	}

	fn parse_tuple_elements(&mut self) -> Result<Vec<Spanned<ASTNode>>, Spanned<ParserError>> {
		self.consume(TokenKind::LPar)?;

		let result = self.parse_elements(&Token::RPar)?;

		self.consume(TokenKind::RPar)?;

		Ok(result)
	}

	fn parse_list_elements(&mut self) -> Result<Vec<Spanned<ASTNode>>, Spanned<ParserError>> {
		self.consume(TokenKind::LBracket)?;

		let result = self.parse_elements(&Token::RBracket)?;

		self.consume(TokenKind::RBracket)?;

		Ok(result)
	}

	fn parse_list(&mut self) -> Result<ASTNode, Spanned<ParserError>> {
		let result = self.parse_list_elements()?;

		Ok(ASTNode::List(result))
	}

	fn parse_function_def(&mut self) -> Result<Spanned<ASTNode>, Spanned<ParserError>> {
		let is_global = match self.peek(0) {
			Some(wmatch!(Token::Global)) => {
				self.consume_next();

				true
			},
			Some(_) => false,
			None => return Ok(
				Spanned::from_with_span(ASTNode::None, 0) // fix: span
			)
		};

		self.consume(TokenKind::Def)?;

		let Some(name_token @ wmatch!(Token::Word(name))) = self.peek(0) else {
			return Err(self.expected(TokenKind::Word))
		};

		let span = name_token.span;

		let name = Spanned::from_with_span(
			name.clone(),
			name_token.span
		);

		self.consume_next();
		self.consume(TokenKind::LPar)?;

		let mut args = Vec::new();

		while let Some(kind) = self.peek(0) {
			match &kind.val {
				Token::Word(arg) => {
					args.push(
						Spanned::from_with_span(
							arg.clone(),
							kind.span
						)
					);

					self.consume_next();
				}
				Token::Comma => {
					self.consume_next();
				},
				_ => break,
			}
		}

		self.consume(TokenKind::RPar)?;

		let block = self.parse(BinaryOp::MAX_PRECEDENCE)?;

		Ok(
			Spanned::from_with_span(
				ASTNode::FunctionDefinition {
					name: Some(name),
					args,
					body: Box::new(block),
					is_global,
				},
				span
			)
		)
	}

	fn parse_statement(&mut self) -> Result<Spanned<ASTNode>, Spanned<ParserError>> {
		let Some(cur) = self.peek(0) else {
			return Ok(
				Spanned::from_with_span(ASTNode::None, 0) // fix: span
			)
		};

		let span = cur.span;

		let result = match (cur, self.peek(1)) {
			(wmatch!(Token::Return), ..) => {
				self.consume_next();

				let value = self.parse(BinaryOp::MAX_PRECEDENCE)?;

				ASTNode::Return(Box::new(value))
			}

			(wmatch!(Token::Break), ..) => {
				self.consume_next();

				let value = self.parse(BinaryOp::MAX_PRECEDENCE)?;

				ASTNode::Break(Box::new(value))
			}

			(wmatch!(Token::If), ..) => {
				self.consume_next();

				let condition = self.parse(BinaryOp::MAX_PRECEDENCE)?;
				let block = self.parse(BinaryOp::MAX_PRECEDENCE)?;

				if let Some(wmatch!(Token::Else)) = self.peek(0) {
					self.consume_next();

					let block_else = self.parse(BinaryOp::MAX_PRECEDENCE)?;

					ASTNode::If {
						condition: Box::new(condition),
						body: Box::new(block),
						else_body: Some(Box::new(block_else))
					}
				} else {
					ASTNode::If {
						condition: Box::new(condition),
						body: Box::new(block),
						else_body: None
					}
				}
			}

			(wmatch!(Token::While), ..) => {
				self.consume_next();

				let condition = self.parse(BinaryOp::MAX_PRECEDENCE)?;
				let block = self.parse(BinaryOp::MAX_PRECEDENCE)?;

				match self.peek(0) {
					Some(wmatch!(Token::Else)) => {
						self.consume_next();

						let block_else = self.parse(BinaryOp::MAX_PRECEDENCE)?;

						ASTNode::While {
							condition: Box::new(condition),
							body: Box::new(block),
							else_body: Some(Box::new(block_else))
						}
					}

					_ => ASTNode::While {
							condition: Box::new(condition),
							body: Box::new(block),
							else_body: None
						}
				}
			}

			(wmatch!(Token::Global), Some(wmatch!(Token::Def))) |
			(wmatch!(Token::Def), Some(wmatch!(Token::Word(_)))) => {
				return self.parse_function_def()
			}

			(wmatch!(Token::Def), Some(wmatch!(Token::LPar))) => {
				self.consume_next();
				self.consume(TokenKind::LPar)?;

				let mut args = Vec::new();

				while let Some(kind) = self.peek(0) {
					match &**kind {
						Token::Word(arg) => {
							args.push(
								Spanned::from_with_span(
									arg.clone(),
									kind.span
								)
							);

							self.consume_next();
						}
						Token::Comma => {
							self.consume_next();
						},
						_ => break,
					}
				}

				self.consume(TokenKind::RPar)?;

				let block = self.parse(BinaryOp::MAX_PRECEDENCE)?;

				ASTNode::FunctionDefinition {
					name: None,
					args,
					body: Box::new(block),
					is_global: false
				}
			}

			(wmatch!(Token::LBrace), ..) => {
				self.parse_block()?.val
			}

			(kind, ..) => {
				return Err(
					Spanned::from_with_span(
						ParserError::ExpectedExpression {
							found: kind.val.clone()
						},
						span
					)
				)
			},
		};

		Ok(
			Spanned::from_with_span(result, span)
		)
	}

	fn parse_unary(&mut self) -> Result<Spanned<ASTNode>, Spanned<ParserError>> {
		let Some(cur) = self.peek(0) else {
			return Ok(
				Spanned::from_with_span(ASTNode::None, 0) // fix: span
			)
		};

		let span = cur.span;

		let result = match (cur, self.peek(1)) {
			// (&Token {token_type: TokenType::Everything, column, row}, ..) => {
			// 	self.consume_next();

			// 	Ok(Some(
			// 		ASTNode { value: ASTNodeEnum::Everything, row, column }
			// 	))
			// }

			(wmatch!(Token::True), ..) => {
				self.consume_next();

				ASTNode::Boolean(true)
			}

			(wmatch!(Token::False), ..) => {
				self.consume_next();

				ASTNode::Boolean(false)
			}
			(&wmatch!(Token::Number(value)), _) => {
				self.consume_next();

				ASTNode::Number(value)
			}

			(&wmatch!(Token::Float(value)), _) => {
				self.consume_next();

				ASTNode::Float(value)
			}

			(wmatch!(Token::String(value)), ..) => {
				let value = value.clone();

				self.consume_next();

				ASTNode::String(value)
			}

			// TODO: сделать генерацию парсинга унарных операторов

			(wmatch!(Token::Plus), ..) => {
				self.consume_next();

				return self.parse_unary()
			}

			(wmatch!(Token::Minus), ..) => {
				self.consume_next();

				let value = self.parse_unary()?;

				ASTNode::Unary {
					op: UnaryOp::Minus,
					value: Box::new(value)
				}
			}

			(wmatch!(Token::Exclamation), ..) => {
				self.consume_next();

				let value = self.parse_unary()?;

				ASTNode::Unary {
					op: UnaryOp::Not,
					value: Box::new(value)
				}
			}

			(wmatch!(Token::LPar), ..) => {
				self.consume_next();

				let result = self.parse(BinaryOp::MAX_PRECEDENCE)?;

				self.consume(TokenKind::RPar)?;

				result.val
			}

			(wmatch!(Token::LBracket), ..) => {
				self.parse_list()?
			}

			(wmatch!(Token::Word(word)), Some(wmatch!(Token::LPar))) => {
				let name = word.to_owned();

				self.consume_next();

				let args = self.parse_tuple_elements()?;

				ASTNode::Function {
					name,
					args
				}
			}

			(wmatch!(Token::Word(word)), ..) => {
				let word = word.to_owned();

				self.consume_next();

				ASTNode::Variable(word)
			}

			_ => return self.parse_statement(),
		};

		Ok(
			Spanned::from_with_span(result, span)
		)
	}

	fn parse(&mut self, precedence: usize) -> Result<Spanned<ASTNode>, Spanned<ParserError>> {
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

			let op_span = x.span;

			self.consume_next();

			let left = mem::take(cur);
			let right = self.parse(right_precedence)?;

			*cur = Spanned::from_with_span(
				ASTNode::Binary {
					left: Box::new(left),
					op: Spanned::from_with_span(
						op,
						op_span
					),
					right: Box::new(right)
				},
				span
			);
		}

		Ok(first)
	}
}
