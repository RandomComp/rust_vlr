use std::{collections::VecDeque, mem, num::ParseFloatError};

use crate::lexer::{LexerError, OpArity, OpAssoc, Token, TokenType};

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
	// Ternary {
	// 	left: Box<ASTNode>,
	// 	op: TokenType,
	// 	center: Box<ASTNode>,
	// 	right: Box<ASTNode>,
	// },
	Binary {
		left: Box<ASTNode>,
		op: TokenType,
		right: Box<ASTNode>,
	},
	Unary {
		op: TokenType,
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

impl ASTNode {
	pub const NONE: Self = Self {value: ASTNodeEnum::None, row: 0, column: 0};
}

impl std::fmt::Display for ASTNodeEnum {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			ASTNodeEnum::None => write!(f, "()"),
			ASTNodeEnum::Everything => write!(f, "..."),
			ASTNodeEnum::Boolean(value) => write!(f, "{}", value),
			ASTNodeEnum::Number(value) => write!(f, "{}", value),
			ASTNodeEnum::String(text) => write!(f, "\"{}\"", text),
			ASTNodeEnum::Variable(name) => write!(f, "{}", name),
			ASTNodeEnum::Tuple(values) => {
				for (i, value) in values.iter().enumerate() {
					if i < values.len() - 1 {
						write!(f, "{}, ", value)?;
					} else {
						write!(f, "{}", value)?;
					}
				}

				Ok(())
			},
			ASTNodeEnum::Block(values) => {
				write!(f, "{{ ")?;

				for value in values {
					write!(f, "{}; ", value)?;
				}

				write!(f, "}}")?;

				Ok(())
			},
			ASTNodeEnum::If { condition, block, block_else: Some(block_else) } => {
				write!(f, "if {} {} else {}", condition, block, block_else)
			},
			ASTNodeEnum::If { condition, block, block_else: None } => {
				write!(f, "if {} {}", condition, block)
			},
			ASTNodeEnum::Break(value) => {
				write!(f, "break {}", value)
			},
			ASTNodeEnum::While { condition, block, block_else: Some(block_else) } => {
				write!(f, "while {} {} else {}", condition, block, block_else)
			},
			ASTNodeEnum::While { condition, block, block_else: None } => {
				write!(f, "while {} {}", condition, block)
			},
			// ASTNodeEnum::Ternary { left, op, center, right } =>
			// 	write!(f, "({} {} {} {} {})", left, op, center, op, right),
			ASTNodeEnum::Binary { left, op, right } =>
				write!(f, "({} {} {})", left, op, right),
			ASTNodeEnum::Unary {op, value} =>
				write!(f, "{}({})", op, value),
			ASTNodeEnum::Function {name, arg} => {
				write!(f, "{}({})", name, arg)
			},
			ASTNodeEnum::FunctionDefinition { name: Some(name), args, block } => {
				write!(f, "def {}(", name)?;

				for (i, arg) in args.iter().enumerate() {
					if i > 0 {
						write!(f, ", ")?;
					}

					write!(f, "{}", arg)?;
				}

				write!(f, ") {}", block)?;

				Ok(())
			},
			ASTNodeEnum::FunctionDefinition { name: None, args, block: _ } => {
				write!(f, "anonymous def(")?;

				for (i, arg) in args.iter().enumerate() {
					if i > 0 {
						write!(f, ", ")?;
					}

					write!(f, "{}", arg)?;
				}

				Ok(())
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
		Self {tokens: tokens, index: 0}
	}

	fn peek(&self, offset: usize) -> Option<&Token> {
		self.tokens.get(self.index + offset)
	}

	fn skip(&mut self, expected_token_type: TokenType) {
		loop {
			match self.peek(0) {
				Some(Token { token_type, column: _, row: _ }) if *token_type != expected_token_type => break,
				Some(Token { token_type: _, column: _, row: _ }) => {
					self.index += 1;
				},
				None => break,
			}
		}
	}

	fn consume_next(&mut self) -> Option<&Token> {
		let result = self.tokens.get(self.index);

		self.index += 1;

		result
	}

	fn consume(&mut self, token_type: TokenType) -> Result<Token, ParserError> {
		match self.peek(0) {
			Some(x) if x.token_type == token_type => {
				let res = x.clone();

				self.consume_next();

				Ok(res)
			},
			Some(Token { token_type: found, row, column }) =>
				Err(ParserError::ExpectedToken { row: *row, column: *column, token_type: token_type, found: found.clone() }),
			None => panic!("Outside of array")
		}
	}

	fn _token_exists_until_token(&self, expected_token_type: TokenType, until_token_type: TokenType) -> bool {
		let mut index = 0;

		loop {
			match self.peek(index) {
				Some(Token { token_type, column, row }) if *token_type == until_token_type => break false,
				Some(Token { token_type, column, row }) if *token_type == expected_token_type => break true,
				Some(Token { token_type: _, column: _, row: _ }) => index += 1,
				None => break false,
			}
		}
	}

	fn parse_block(&mut self) -> Result<Option<ASTNode>, ParserError> {
		self.consume(TokenType::TokenLBrace)?;

		let result = self.parse_instructions();

		self.consume(TokenType::TokenRBrace)?;

		result
	}

	pub fn parse_instructions(&mut self) -> Result<Option<ASTNode>, ParserError> {
		let (row, column) = match self.peek(0) {
			Some(Token { token_type: _, column, row }) => (*row, *column),
			None => return Ok(None),
		};

		let mut result: VecDeque<ASTNode> = VecDeque::new();

		loop {
			match self.peek(0) {
				Some(Token { token_type: TokenType::TokenRBrace, column: _, row: _ }) |
				Some(Token { token_type: TokenType::TokenEOF, column: _, row: _ }) => {
					break
				},
				_ => {},
			}

			let value = self.parse(TokenType::MAX_PRECEDENCE)?;

			result.push_back(value);

			match self.peek(0) {
				Some(Token { token_type: TokenType::TokenSemicolon, column: _, row: _ }) => {
					self.skip(TokenType::TokenSemicolon);
				},
				_ => {},
			}
		}

		if result.len() == 1 {
			Ok(result.pop_front())
		} else {
			Ok(Some(
				ASTNode { value: ASTNodeEnum::Block(result), row: row, column: column }
			))
		}
	}

	fn parse_tuple(&mut self) -> Result<Option<ASTNode>, ParserError> {
		self.consume(TokenType::TokenLPar)?;

		let mut result: Vec<ASTNode> = Vec::new();

		loop {
			match self.peek(0) {
				Some(&Token { token_type: TokenType::TokenRPar, column, row }) => {
					self.consume(TokenType::TokenRPar)?;

					return Ok(Some(ASTNode { value: ASTNodeEnum::Tuple(result), row: row, column: column }))
				},
				_ => {},
			}

			let value = self.parse(TokenType::MAX_PRECEDENCE)?;

			match self.peek(0) {
				Some(Token { token_type: TokenType::TokenComma, column: _, row: _ }) => {
					result.push(value);

					self.consume_next();
				},
				Some(Token { token_type: TokenType::TokenRPar, column, row }) if result.len() == 0 => {
					self.consume(TokenType::TokenRPar)?;

					return Ok(Some(value))
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
	// 				ASTNode { value: ASTNodeEnum::Assignment {name: name, value: Box::new(value)}, row: row, column: column }
	// 			))
	// 		},
	// 		Some(_) => return Ok(Some(self.parse(TokenType::MAX_PRECEDENCE)?)),
	// 		None => return Ok(None),
	// 	}
	// }

	fn parse_unary(&mut self) -> Result<Option<ASTNode>, ParserError> {
		let cur = match self.peek(0) {
			Some(x) => x,
			None => return Ok(None),
		};

		match (cur, self.peek(1)) {
			(&Token {token_type: TokenType::TokenTrue, column, row}, _) => {
				self.consume_next();

				Ok(Some(
					ASTNode { value: ASTNodeEnum::Boolean(true), row: row, column: column }
				))
			}

			(&Token {token_type: TokenType::TokenFalse, column, row}, _) => {
				self.consume_next();

				Ok(Some(
					ASTNode { value: ASTNodeEnum::Boolean(false), row: row, column: column }
				))
			}

			(&Token {token_type: TokenType::TokenBreak, column, row}, _) => {
				self.consume_next();

				let value = self.parse(TokenType::MAX_PRECEDENCE)?;

				Ok(Some(
					ASTNode { value: ASTNodeEnum::Break(Box::new(value)), row: row, column: column }
				))
			}

			(&Token {token_type: TokenType::TokenIf, column, row}, _) => {
				self.consume_next();

				let condition = self.parse(TokenType::MAX_PRECEDENCE)?;
				let block = self.parse(TokenType::MAX_PRECEDENCE)?;

				match self.peek(0) {
					Some(&Token { token_type: TokenType::TokenElse, row, column }) => {
						self.consume_next();

						let block_else = self.parse(TokenType::MAX_PRECEDENCE)?;

						Ok(Some(
							ASTNode { value: ASTNodeEnum::If { condition: Box::new(condition), block: Box::new(block), block_else: Some(Box::new(block_else)) }, row: row, column: column }
						))
					}

					_ => Ok(Some(
						ASTNode { value: ASTNodeEnum::If { condition: Box::new(condition), block: Box::new(block), block_else: None }, row: row, column: column }
					))
				}
			}

			(&Token {token_type: TokenType::TokenWhile, column, row}, _) => {
				self.consume_next();

				let condition = self.parse(TokenType::MAX_PRECEDENCE)?;
				let block = self.parse(TokenType::MAX_PRECEDENCE)?;

				match self.peek(0) {
					Some(&Token { token_type: TokenType::TokenElse, row, column }) => {
						self.consume_next();

						let block_else = self.parse(TokenType::MAX_PRECEDENCE)?;

						Ok(Some(
							ASTNode { value: ASTNodeEnum::While { condition: Box::new(condition), block: Box::new(block), block_else: Some(Box::new(block_else)) }, row: row, column: column }
						))
					}

					_ => Ok(Some(
						ASTNode { value: ASTNodeEnum::While { condition: Box::new(condition), block: Box::new(block), block_else: None }, row: row, column: column }
					))
				}
			}

			(&Token {token_type: TokenType::TokenDef, column, row}, Some(Token { token_type: TokenType::TokenWord(name), row: _, column: _ })) => {
				let name = name.clone();

				self.consume_next();
				self.consume_next();
				self.consume(TokenType::TokenLPar)?;

				let mut args = Vec::new();

				loop {
					match self.peek(0) {
						Some(Token { token_type: TokenType::TokenWord(arg), column: _, row: _ }) => {
							args.push(arg.clone());

							self.consume_next();
						},
						_ => break,
					}

					match self.peek(0) {
						Some(Token { token_type: TokenType::TokenComma, column: _, row: _ }) => {
							self.consume_next();

							continue
						},
						_ => break,
					}
				}

				self.consume(TokenType::TokenRPar)?;

				let block = self.parse(TokenType::MAX_PRECEDENCE)?;

				Ok(Some(ASTNode { value:
						ASTNodeEnum::FunctionDefinition { name: Some(name), args: args, block: Box::new(block) },
					row: row, column: column }
				))
			}

			(&Token {token_type: TokenType::TokenDef, column, row}, Some(Token { token_type: TokenType::TokenLPar, row: _, column: _ })) => {
				self.consume_next();
				self.consume(TokenType::TokenLPar)?;

				let mut args = Vec::new();

				loop {
					match self.peek(0) {
						Some(Token { token_type: TokenType::TokenWord(arg), column: _, row: _ }) => {
							args.push(arg.clone());

							self.consume_next();
						},
						_ => break,
					}

					match self.peek(0) {
						Some(Token { token_type: TokenType::TokenComma, column: _, row: _ }) => {
							self.consume_next();

							continue
						},
						_ => break,
					}
				}

				self.consume(TokenType::TokenRPar)?;

				let block = self.parse(TokenType::MAX_PRECEDENCE)?;

				Ok(Some(ASTNode { value:
						ASTNodeEnum::FunctionDefinition { name: None, args: args, block: Box::new(block) },
					row: row, column: column }
				))
			}

			(&Token {token_type: TokenType::TokenEverything, column, row}, _) => {
				self.consume_next();

				Ok(Some(
					ASTNode { value: ASTNodeEnum::Everything, row: row, column: column }
				))
			}

			(&Token {token_type: TokenType::TokenNumber(value), column, row}, _) => {
				self.consume_next();

				Ok(Some(
					ASTNode { value: ASTNodeEnum::Number(value), row: row, column: column }
				))
			}

			(Token {token_type: TokenType::TokenString(value), column, row}, _) => {
				let (column, row) = (*column, *row);
				let value = value.clone();

				self.consume_next();

				Ok(Some(
					ASTNode { value: ASTNodeEnum::String(value), row: row, column: column }
				))
			}

			(&Token {token_type: TokenType::TokenPlus, column, row}, _) => {
				self.consume_next();

				let value = match self.parse_unary()? {
					Some(x) => x,
					None => return Ok(None),
				};

				Ok(Some(
					ASTNode { value: ASTNodeEnum::Unary { op: TokenType::TokenPlus, value: Box::new(value) }, row: row, column: column  }
				))
			}

			(&Token {token_type: TokenType::TokenMinus, column, row}, _) => {
				self.consume_next();

				let value = match self.parse_unary()? {
					Some(x) => x,
					None => return Ok(None),
				};

				Ok(Some(
					ASTNode { value: ASTNodeEnum::Unary { op: TokenType::TokenMinus, value: Box::new(value) }, row: row, column: column  }
				))
			}

			(&Token {token_type: TokenType::TokenLPar, column: _, row: _}, _) => {
				self.parse_tuple()
			}

			(&Token {token_type: TokenType::TokenLBrace, column: _, row: _}, _) => {
				self.parse_block()
			}

			(Token {token_type: TokenType::TokenWord(word), column, row}, Some(Token { token_type: TokenType::TokenLPar, column: _, row: _ })) => {
				let (row, column) = (*row, *column);

				let word = word.clone();

				self.consume_next();

				let value = self.parse_tuple()?.unwrap();

				Ok(Some(
					ASTNode { value: ASTNodeEnum::Function { name: word, arg: Box::new(value) }, row: row, column: column }
				))
			}

			(Token {token_type: TokenType::TokenWord(word), column, row}, _) => {
				let (column, row) = (*column, *row);
				let word = word.clone();

				self.consume_next();

				return Ok(Some(
					ASTNode { value: ASTNodeEnum::Variable(word), row: row, column: column }
				))
			}

			(Token {token_type, column, row}, _) =>
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

		loop {
			match self.peek(0) {
				Some(x) if x.token_type.precedence() == Some(precedence) => {
					let op = x.token_type.clone();

					let right_precedence = if x.token_type.assoc() == OpAssoc::Right {precedence} else {precedence - 1};

					let old_cur = mem::replace(cur, ASTNode::NONE);

					match x.token_type.arity() {
						OpArity::Binary => {
							self.consume_next();

							let right = self.parse(right_precedence)?;

							let new_cur = ASTNode { value: ASTNodeEnum::Binary { left: Box::new(old_cur), op: op, right: Box::new(right) }, row: row, column: column };
							*cur = new_cur;
						}

						// OpArity::Ternary => {
						// 	self.consume_next();

						// 	let center = self.parse(right_precedence)?;

						// 	self.consume_next();

						// 	let right = self.parse(right_precedence)?;

						// 	let new_cur = ASTNode { value: ASTNodeEnum::Ternary {
						// 		left: Box::new(old_cur), op: op, center: Box::new(center), right: Box::new(right)
						// 	}, row: row, column: column };

						// 	*cur = new_cur;
						// }
					}
				},
				_ => break
			}
		}

		Ok(first)
	}
}
