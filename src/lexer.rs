use std::{iter::Skip, num::ParseFloatError, str::Chars};

use crate::{res_with_pos, wrapper::{LexerError, Token}};

#[derive(Clone, Debug, PartialEq)]
pub enum RawToken {
	Float(f64),
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
	Neq,
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

macro_rules! gen_code_for_ops {
	($($kind:path => $text:expr);* ) => {
		pub const TOKEN_OPERATORS: &[(Self, &str)] = &[
			$(
				($kind, $text),
			)*
		];

		fn display(&self) -> Option<&'static str> {
			match self {
				$(
					$kind => Some($text),
				)*
				_ => None
			}
		}
	};
}

impl RawToken {
	gen_code_for_ops!(
		Self::Equals => "==";
		Self::Neq => "!=";
		Self::GreatOrEquals => ">=";
		Self::LessOrEquals => "<=";
		Self::Great => ">";
		Self::Less => "<";
		Self::LogicalAnd => "&&";
		Self::LogicalOr => "||";

		Self::Assignment => "=";
		Self::PlusAssignment => "+=";
		Self::MinusAssignment => "-=";
		Self::PowAssignment => "**=";
		Self::MultiplyAssignment => "*=";
		Self::DivideAssignment => "/=";
		Self::RemainderAssignment => "%=";

		Self::Plus => "+";
		Self::Minus => "-";
		Self::Exclamation => "!";
		Self::Pow => "**";
		Self::Multiply => "*";
		Self::Divide => "/";
		Self::Remainder => "%";

		Self::Comma => ",";
		Self::Semicolon => ";";
		Self::LPar => "(";
		Self::RPar => ")";
		Self::LBrace => "{";
		Self::RBrace => "}";

		Self::Everything => "...";
		Self::Range => ".."
	);

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

	fn from_op(expected_op: &str) -> Option<&(Self, &str)> {
		Self::TOKEN_OPERATORS.iter().find(|(_, text)| expected_op.starts_with(text))
	}
}

impl std::fmt::Display for RawToken {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Self::Float(x) => write!(f, "{x}"),
			Self::Word(word) => write!(f, "{word}"),
			Self::String(word) => write!(f, "\"{word}\""),

			Self::True => write!(f, "true"),
			Self::False => write!(f, "false"),

			Self::If => write!(f, "if"),
			Self::Else => write!(f, "else"),

			Self::While => write!(f, "while"),
			Self::For => write!(f, "for"),

			Self::Break => write!(f, "break"),
			Self::Return => write!(f, "return"),

			Self::Def => write!(f, "def"),

			Self::Eof => write!(f, "EOF"),

			_ => f.write_str(self.display().unwrap()),
		}
	}
}

#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum LexerErrorRaw {
	#[error("unknown operation")]
	UnknownOperation,
	#[error("invalid number")]
	InvalidNumber(#[from] ParseFloatError),
	#[error("expected {c}")]
	ExpectedSymbol {c: char},
}

pub struct Lexer<'a> {
	spans: &'a mut Vec<(usize, usize)>,
	expr: Chars<'a>,
	index: usize,

	column: usize, row: usize,
}

impl<'a> Lexer<'a> {
	pub fn new(spans: &'a mut Vec<(usize, usize)>, expr: Chars<'a>) -> Self {
		Lexer {spans, expr, index: 0, column: 0, row: 0}
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

	fn consume(&mut self, expected_c: char) -> Result<(), LexerErrorRaw> {
		match self.peek(0) {
			Some(c) if c == expected_c => {
				self.consume_next(1);

				Ok(())
			},
			_ => Err(LexerErrorRaw::ExpectedSymbol { c: expected_c })
		}
	}

	fn tokenize_operator(&mut self) -> Result<RawToken, LexerErrorRaw> {
		let expected_op = self.get().take(3).collect::<String>();

		let Some((result, text)) = RawToken::from_op(&expected_op) else {
		 	return Err(LexerErrorRaw::UnknownOperation)
		};

		self.consume_next(text.len());

		Ok(result.clone())
	}

	fn tokenize_number(&mut self) -> Result<RawToken, LexerErrorRaw> {
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

		let result = number_str.parse::<f64>()?;

		self.consume_next(word_chr_cnt);

		Ok(RawToken::Float(result))
	}

	fn tokenize_word(&mut self) -> RawToken {
		let mut word_chr_cnt = 0;

		let mut word_str = String::new();

		for c in self.get() {
			if !c.is_ascii_alphanumeric() && c != '_' {
				break;
			}

			word_str.push(c);
			word_chr_cnt += 1;
		}

		if let Some(token_type) = RawToken::get_keyword(word_str.as_str()) {
			self.consume_next(word_chr_cnt);

			token_type
		} else {
			self.consume_next(word_chr_cnt);

			RawToken::Word(word_str)
		}
	}

	fn tokenize_string(&mut self) -> Result<RawToken, LexerErrorRaw> {
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

		self.consume_next(word_chr_cnt);

		self.consume('"')?;

		Ok(RawToken::String(word_str))
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
				None => return Ok(RawToken::Eof.into())
			}
		}

		let span = self.spans.len();
		self.spans.push((self.row, self.column));

		let result = match self.peek(0) {
			Some(c) if c.is_ascii_alphabetic() || c == '_' =>
				Ok(self.tokenize_word()),
			Some('"') =>
				self.tokenize_string(),
			Some(c) if c.is_ascii_digit() =>
				self.tokenize_number(),
			Some(_) =>
				self.tokenize_operator(),

			None => Ok(RawToken::Eof)
		};

		res_with_pos!(result, Token, LexerError, span)
	}

	pub fn tokenize_loop(&mut self) -> Result<Vec<Token>, LexerError> {
		let mut result: Vec<Token> = Vec::new();

		while let token = self.tokenize()? && token != RawToken::Eof {
			result.push(token);
		}

		let span = self.spans.len();
		self.spans.push((self.row, self.column));

		result.push(Token::from(RawToken::Eof).with_pos(span));

		Ok(result)
	}
}
