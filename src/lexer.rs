use std::{fmt, iter::{Skip, zip}, num::ParseFloatError, str::Chars};

#[derive(Clone, Debug, PartialEq)]
pub enum TokenType {
	TokenUndefined,
	TokenNumber(f64),
	TokenWord(String),
	TokenString(String),
	TokenPlus,
	TokenMinus,
	TokenMultiply,
	TokenPow,
	TokenDivide,
	TokenRemainder,
	TokenAssignment,

	TokenTrue, TokenFalse,
	TokenIf,
	TokenElse,
	TokenEverything,

	TokenEquals,
	TokenNotEquals,
	TokenGreat,
	TokenLess,
	TokenGreatOrEquals,
	TokenLessOrEquals,
	TokenLogicalAnd,
	TokenLogicalOr,

	TokenRange,

	TokenComma,
	TokenSemicolon,
	TokenLPar,
	TokenRPar,
	TokenLBrace,
	TokenRBrace,
	TokenEOF,
}

#[derive(Debug, PartialEq)]
pub enum OpAssoc {
	Left,
	Right
}

#[derive(Debug, PartialEq)]
pub enum OpArity {
	Binary,
	// Ternary
}

impl fmt::Display for OpAssoc {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Left => write!(f, "left"),
			Self::Right => write!(f, "right")
		}
	}
}

impl TokenType {
	pub const TOKEN_OPERATORS: &[(Self, &str)] = &[
		(Self::TokenPlus, "+"),
		(Self::TokenMinus, "-"),
		(Self::TokenPow, "**"),
		(Self::TokenMultiply, "*"),
		(Self::TokenDivide, "/"),
		(Self::TokenRemainder, "%"),

		(Self::TokenEquals, "=="),
		(Self::TokenNotEquals, "!="),
		(Self::TokenGreatOrEquals, ">="),
		(Self::TokenLessOrEquals, "<="),
		(Self::TokenGreat, ">"),
		(Self::TokenLess, "<"),
		(Self::TokenLogicalAnd, "&&"),
		(Self::TokenLogicalOr, "||"),

		(Self::TokenAssignment, "="),
		(Self::TokenComma, ","),
		(Self::TokenSemicolon, ";"),
		(Self::TokenLPar, "("),
		(Self::TokenRPar, ")"),
		(Self::TokenLBrace, "{"),
		(Self::TokenRBrace, "}"),

		(Self::TokenEverything, "..."),
		(Self::TokenRange, ".."),
	];

	pub const TOKEN_KEYWORDS: &[(Self, &str)] = &[
		(Self::TokenTrue, "true"),
		(Self::TokenFalse, "false"),

		(Self::TokenIf, "if"),
		(Self::TokenElse, "else"),
	];

	pub const MAX_PRECEDENCE: usize = 6;

	pub fn precedence(&self) -> Option<usize> {
		match self {
			Self::TokenRange => Some(1),
			Self::TokenPow => Some(2),
			Self::TokenRemainder => Some(3),
			Self::TokenDivide => Some(3),
			Self::TokenMultiply => Some(3),
			Self::TokenPlus => Some(4),
			Self::TokenMinus => Some(4),

			Self::TokenEquals => Some(5),
			Self::TokenNotEquals => Some(5),
			Self::TokenGreatOrEquals => Some(5),
			Self::TokenLessOrEquals => Some(5),
			Self::TokenGreat => Some(5),
			Self::TokenLess => Some(5),

			Self::TokenLogicalAnd => Some(6),
			Self::TokenLogicalOr => Some(6),

			_ => None
		}
	}

	pub fn assoc(&self) -> OpAssoc {
		match self {
			Self::TokenPow => OpAssoc::Right,
			_ => OpAssoc::Left,
		}
	}

	pub fn arity(&self) -> OpArity {
		match self {
			_ => OpArity::Binary,
		}
	}
}

impl std::fmt::Display for TokenType {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::TokenUndefined => write!(f, "undefined"),
			Self::TokenNumber(x) => write!(f, "{}", x),
			Self::TokenWord(word) => write!(f, "{}", word),
			Self::TokenString(word) => write!(f, "\"{}\"", word),
			Self::TokenPlus => write!(f, "+"),
			Self::TokenMinus => write!(f, "-"),
			Self::TokenMultiply => write!(f, "*"),
			Self::TokenPow => write!(f, "**"),
			Self::TokenDivide => write!(f, "/"),
			Self::TokenRemainder => write!(f, "%"),

			Self::TokenTrue => write!(f, "true"),
			Self::TokenFalse => write!(f, "false"),
			Self::TokenIf => write!(f, "if"),
			Self::TokenElse => write!(f, "else"),
			Self::TokenEverything => write!(f, ".."),

			Self::TokenEquals => write!(f, "=="),
			Self::TokenNotEquals => write!(f, "!="),
			Self::TokenGreatOrEquals => write!(f, ">="),
			Self::TokenLessOrEquals => write!(f, "<="),
			Self::TokenGreat => write!(f, ">"),
			Self::TokenLess => write!(f, "<"),

			Self::TokenLogicalAnd => write!(f, "&&"),
			Self::TokenLogicalOr => write!(f, "||"),

			Self::TokenRange => write!(f, ".."),

			Self::TokenAssignment => write!(f, "="),
			Self::TokenComma => write!(f, ","),
			Self::TokenSemicolon => write!(f, ";"),
			Self::TokenLPar => write!(f, "("),
			Self::TokenRPar => write!(f, ")"),
			Self::TokenLBrace => write!(f, "{{"),
			Self::TokenRBrace => write!(f, "}}"),
			Self::TokenEOF => write!(f, "EOF"),
		}
	}
}

#[derive(Clone)]
pub struct Token {
	pub token_type: TokenType,
	pub row: usize, pub column: usize,
}

impl std::fmt::Display for Token {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{}", self.token_type)
	}
}

impl Token {
	fn new(token_type: TokenType, row: usize, column: usize) -> Self {
		Self {token_type: token_type, row: row, column: column}
	}
}

#[derive(Debug, thiserror::Error)]
pub enum LexerError {
	#[error("line {row} at {column}: unknown operation")]
	UnknownOperation {row: usize, column: usize},
	#[error("line {row} at {column}: invalid number")]
	InvalidNumber {row: usize, column: usize, e: ParseFloatError},
	// #[error("line {row} at {column}: multiple decimal points")]
	// MultipleDecimalPoints {row: usize, column: usize},
	#[error("line {row} at {column}: expected {c}")]
	ExpectedSymbol {row: usize, column: usize, c: char},
}

pub struct Lexer<'a> {
	expr: Chars<'a>,
	index: usize,

	column: usize, row: usize,
}

impl<'a> Lexer<'a> {
	pub fn new(expr: Chars<'a>) -> Self {
		Lexer {expr: expr, index: 0, column: 0, row: 0}
	}

	fn get(&self) -> Skip<Chars<'a>> {
		self.expr.clone().skip(self.index)
	}

	fn peek(&mut self, offset: usize) -> Option<char> {
		self.expr.clone().nth(self.index + offset)
	}

	fn consume_next(&mut self, len: usize) {
		self.index += len;
		self.column += len;
	}

	fn consume(&mut self, expected_c: char) -> Result<(), LexerError> {
		match self.peek(0) {
			Some(c) if c == expected_c => {
				self.consume_next(1);

				Ok(())
			},
			_ => Err(LexerError::ExpectedSymbol { row: self.row, column: self.column, c: expected_c })
		}
	}

	fn tokenize_operator(&mut self) -> Result<Token, LexerError> {
		let (row, column) = (self.row, self.column);

		let mut result = &TokenType::TokenUndefined;

		for (token_type, op) in TokenType::TOKEN_OPERATORS {
			let op_len = op.len();

			let expected_op = self.get().take(op_len).collect::<String>();

			if expected_op.eq(op) {
				result = token_type;

				self.consume_next(op_len);

				break;
			}
		}

		if *result == TokenType::TokenUndefined {
			 return Err(LexerError::UnknownOperation {row: row, column: column})
		}

		Ok(Token::new(result.clone(), row, column))
	}

	fn tokenize_number(&mut self) -> Result<Token, LexerError> {
		let (row, column) = (self.row, self.column);

		let mut word_chr_cnt = 0;

		let mut dots = 0;

		let mut number_str = String::new();

		for c in self.get() {
			if c == '.' && dots == 0 {
				dots += 1;
			} else if (c == '.' && !c.is_ascii_digit()) || !c.is_ascii_digit() {
				break;
			}

			number_str.push(c);

			word_chr_cnt += 1;
		}

		match number_str.chars().last() {
			Some(x) if x == '.' => {
				number_str.pop();

				word_chr_cnt -= 1
			},
			_ => {},
		};

		let result = match number_str.parse::<f64>() {
			Ok(x) => x,
			Err(e) => Err(LexerError::InvalidNumber {row: self.row, column: self.column, e: e})?
		};

		self.consume_next(word_chr_cnt);

		Ok(Token::new(TokenType::TokenNumber(result), row, column))
	}

	fn tokenize_word(&mut self) -> Result<Token, LexerError> {
		let mut word_chr_cnt = 0;

		let mut word_str = String::new();

		for c in self.get() {
			if !c.is_ascii_alphanumeric() && c != '_' {
				break;
			}

			word_str.push(c);
			word_chr_cnt += 1;
		}

		match TokenType::TOKEN_KEYWORDS.iter().find(|&(_, keyword)| *keyword == word_str) {
			Some((token_type, _)) => {
				self.consume_next(word_chr_cnt);

				Ok(Token::new(token_type.clone(), self.column, self.row))
			},
			None => {
				self.consume_next(word_chr_cnt);

				Ok(Token::new(TokenType::TokenWord(word_str), self.column, self.row))
			}
		}
	}

	fn tokenize_string(&mut self) -> Result<Token, LexerError> {
		let (row, column) = (self.row, self.column);

		self.consume('"')?;

		let mut word_chr_cnt: usize = 0;

		let mut word_str = String::new();

		for c in self.get() {
			if c == '"' {
				break;
			}

			word_str.push(c);
			word_chr_cnt += 1;
		}

		let result = Token::new(TokenType::TokenString(word_str), row, column);

		self.consume_next(word_chr_cnt);

		self.consume('"')?;

		Ok(result)
	}

	fn skip_comments(&mut self) {
		let mut index = self.index;

		for (c, c_next) in zip(self.get(), self.get().skip(1)) {
			index += 1;

			if c == '\n' && c_next != '#' {
				break;
			}
		}

		self.column = 0;
		self.row += 1;
		self.index = index;
	}

	fn skip_spaces(&mut self) {
		let (mut row, mut column, mut index) = (self.row, self.column, self.index);

		for c in self.get() {
			if c == '\n' {
				row += 1;
				column = 0;
			} else if c.is_whitespace() {
				column += 1;
			} else {
				break;
			}

			index += 1;
		}

		self.row = row;
		self.column = column;
		self.index = index;
	}

	fn tokenize(&mut self) -> Result<Token, LexerError> {
		loop {
			match self.peek(0) {
				Some(c) if c.is_whitespace() =>
		 			self.skip_spaces(),
				Some(c) if c == '#' =>
					self.skip_comments(),
				Some(_) => break,
				None => return Ok(Token::new(TokenType::TokenEOF, self.column, self.row))
			}
		}

		match self.peek(0) {
			Some(c) if c.is_ascii_alphabetic() =>
				self.tokenize_word(),
			Some(c) if c == '"' =>
				self.tokenize_string(),
			Some(c) if c.is_ascii_digit() =>
				self.tokenize_number(),
			Some(_) =>
				self.tokenize_operator(),

			None => Ok(Token::new(TokenType::TokenEOF, self.column, self.row))
		}
	}

	pub fn tokenize_loop(&mut self) -> Result<Vec<Token>, LexerError> {
		let mut result: Vec<Token> = Vec::new();

		loop {
			let token = self.tokenize()?;
			let token_type = token.token_type.clone();

			result.push(token);

			if token_type == TokenType::TokenEOF {
				break;
			}
		}

		Ok(result)
	}
}
