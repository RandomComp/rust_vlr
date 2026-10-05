use std::{iter::Skip, num::ParseFloatError, str::Chars};

#[derive(Clone, Debug, PartialEq)]
pub enum TokenType {
	Undefined,
	Number(f64),
	Word(String),
	String(String),
	Plus,
	Minus,
	Exclamation,
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

	True, False,
	If, Else,
	While, For,
	Break,
	Return,
	Def,
	Everything,

	Equals,
	NotEquals,
	Great,
	Less,
	GreatOrEquals,
	LessOrEquals,
	LogicalAnd,
	LogicalOr,

	Range,

	Comma,
	Semicolon,
	LPar,
	RPar,
	LBrace,
	RBrace,
	Eof,
}

impl TokenType {
	pub const TOKEN_OPERATORS: &[(Self, &str)] = &[
		(Self::Equals, "=="),
		(Self::NotEquals, "!="),
		(Self::GreatOrEquals, ">="),
		(Self::LessOrEquals, "<="),
		(Self::Great, ">"),
		(Self::Less, "<"),
		(Self::LogicalAnd, "&&"),
		(Self::LogicalOr, "||"),

		(Self::Assignment, "="),
		(Self::PlusAssignment, "+="),
		(Self::MinusAssignment, "-="),
		(Self::PowAssignment, "**="),
		(Self::MultiplyAssignment, "*="),
		(Self::DivideAssignment, "/="),
		(Self::RemainderAssignment, "%="),

		(Self::Plus, "+"),
		(Self::Minus, "-"),
		(Self::Exclamation, "!"),
		(Self::Pow, "**"),
		(Self::Multiply, "*"),
		(Self::Divide, "/"),
		(Self::Remainder, "%"),

		(Self::Comma, ","),
		(Self::Semicolon, ";"),
		(Self::LPar, "("),
		(Self::RPar, ")"),
		(Self::LBrace, "{"),
		(Self::RBrace, "}"),

		(Self::Everything, "..."),
		(Self::Range, ".."),
	];

	pub const MAX_PRECEDENCE: usize = 7;

	fn get_keyword(word: &str) -> Option<Self> {
		match word {
			"true" => Some(Self::True),
			"false" => Some(Self::False),
			"if" => Some(Self::If),
			"else" => Some(Self::Else),
			"while" => Some(Self::While),
			"for" => Some(Self::For),
			"def" => Some(Self::Def),
			"break" => Some(Self::Break),
			"return" => Some(Self::Return),
			_ => None,
		}
	}
}

impl std::fmt::Display for TokenType {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::Undefined => write!(f, "undefined"),
			Self::Number(x) => write!(f, "{x}"),
			Self::Word(word) => write!(f, "{word}"),
			Self::String(word) => write!(f, "\"{word}\""),

			Self::Assignment => write!(f, "="),
			Self::PlusAssignment => write!(f, "+="),
			Self::MinusAssignment => write!(f, "-="),
			Self::PowAssignment => write!(f, "**="),
			Self::MultiplyAssignment => write!(f, "*="),
			Self::DivideAssignment => write!(f, "/="),
			Self::RemainderAssignment => write!(f, "%="),

			Self::Plus => write!(f, "+"),
			Self::Minus => write!(f, "-"),
			Self::Exclamation => write!(f, "!"),
			Self::Multiply => write!(f, "*"),
			Self::Pow => write!(f, "**"),
			Self::Divide => write!(f, "/"),
			Self::Remainder => write!(f, "%"),

			Self::True => write!(f, "true"),
			Self::False => write!(f, "false"),

			Self::If => write!(f, "if"),
			Self::Else => write!(f, "else"),

			Self::While => write!(f, "while"),
			Self::For => write!(f, "for"),

			Self::Break => write!(f, "break"),
			Self::Return => write!(f, "return"),

			Self::Def => write!(f, "def"),
			Self::Everything => write!(f, "..."),

			Self::Equals => write!(f, "=="),
			Self::NotEquals => write!(f, "!="),
			Self::GreatOrEquals => write!(f, ">="),
			Self::LessOrEquals => write!(f, "<="),
			Self::Great => write!(f, ">"),
			Self::Less => write!(f, "<"),

			Self::LogicalAnd => write!(f, "&&"),
			Self::LogicalOr => write!(f, "||"),

			Self::Range => write!(f, ".."),

			Self::Comma => write!(f, ","),
			Self::Semicolon => write!(f, ";"),
			Self::LPar => write!(f, "("),
			Self::RPar => write!(f, ")"),
			Self::LBrace => write!(f, "{{"),
			Self::RBrace => write!(f, "}}"),
			Self::Eof => write!(f, "EOF"),
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
		Self {token_type, row, column}
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
		Lexer {expr, index: 0, column: 0, row: 0}
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

		let mut result = &TokenType::Undefined;

		for (token_type, op) in TokenType::TOKEN_OPERATORS {
			let op_len = op.len();

			let expected_op = self.get().take(op_len).collect::<String>();

			if expected_op.eq(op) {
				result = token_type;

				self.consume_next(op_len);

				break;
			}
		}

		if *result == TokenType::Undefined {
			 return Err(LexerError::UnknownOperation {row, column})
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

		if let Some(x) = number_str.chars().last() && x == '.' {
			number_str.pop();

			word_chr_cnt -= 1;
		}

		let result = match number_str.parse::<f64>() {
			Ok(x) => x,
			Err(e) => Err(LexerError::InvalidNumber {row: self.row, column: self.column, e})?
		};

		self.consume_next(word_chr_cnt);

		Ok(Token::new(TokenType::Number(result), row, column))
	}

	fn tokenize_word(&mut self) -> Token {
		let mut word_chr_cnt = 0;

		let mut word_str = String::new();

		for c in self.get() {
			if !c.is_ascii_alphanumeric() && c != '_' {
				break;
			}

			word_str.push(c);
			word_chr_cnt += 1;
		}

		if let Some(token_type) = TokenType::get_keyword(word_str.as_str()) {
			self.consume_next(word_chr_cnt);

			Token::new(token_type, self.row, self.column)
		} else {
			self.consume_next(word_chr_cnt);

			Token::new(TokenType::Word(word_str), self.row, self.column)
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

		let result = Token::new(TokenType::String(word_str), row, column);

		self.consume_next(word_chr_cnt);

		self.consume('"')?;

		Ok(result)
	}

	fn skip_comments(&mut self) {
		let (mut row, mut column, mut index) = (self.row, self.column, self.index);

		let mut next_line = false;

		for c in self.get() {
			if next_line && c != '#' {
				break;
			}

			next_line = false;

			column += 1;
			index += 1;

			if c == '\n' {
				column = 0;
				row += 1;

				next_line = true;
			}
		}

		self.column = column;
		self.row = row;
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
				Some('#') =>
					self.skip_comments(),
				Some(_) => break,
				None => return Ok(Token::new(TokenType::Eof, self.row, self.column))
			}
		}

		match self.peek(0) {
			Some(c) if c.is_ascii_alphabetic() =>
				Ok(self.tokenize_word()),
			Some('"') =>
				self.tokenize_string(),
			Some(c) if c.is_ascii_digit() =>
				self.tokenize_number(),
			Some(_) =>
				self.tokenize_operator(),

			None => Ok(Token::new(TokenType::Eof, self.row, self.column))
		}
	}

	pub fn tokenize_loop(&mut self) -> Result<Vec<Token>, LexerError> {
		let mut result: Vec<Token> = Vec::new();

		loop {
			let token = self.tokenize()?;
			let token_type = token.token_type.clone();

			result.push(token);

			if token_type == TokenType::Eof {
				break;
			}
		}

		Ok(result)
	}
}
