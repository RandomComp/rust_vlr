use std::{iter::Skip, num::{IntErrorKind, ParseFloatError, ParseIntError}, str::Chars};

use crate::{res_with_pos, wrapper::Spanned};

macro_rules! gen_code_for_ops_and_for_display {
	(
		operators => { $($kind:tt => $text:expr),* }
		keywords => { $($kind_word:tt => $text_word:expr),* }
		literals => { $($kind_other:tt($kind_parametrs:ty) => $text_other:expr),* }
		special => { $($kind_special:tt => $text_special:expr),* }
	) => {
		#[derive(Copy, Clone, Debug, PartialEq)]
		pub enum TokenKind {
			$(
				$kind,
			)*

			$(
				$kind_word,
			)*

			$(
				$kind_other,
			)*

			$(
				$kind_special,
			)*
		}

		#[derive(Clone, Debug, PartialEq)]
		pub enum Token {
			$(
				$kind,
			)*

			$(
				$kind_word,
			)*

			$(
				$kind_other ( $kind_parametrs ),
			)*

			$(
				$kind_special,
			)*
		}

		impl From<&Token> for TokenKind {
			fn from(from: &Token) -> Self {
				match from {
					$(
						Token::$kind => Self::$kind,
					)*

					$(
						Token::$kind_word => Self::$kind_word,
					)*

					$(
						Token::$kind_other(..) => Self::$kind_other,
					)*

					$(
						Token::$kind_special => Self::$kind_special,
					)*
				}
			}
		}

		impl Token {
			const TOKEN_OPERATORS: &[(Self, &str)] = &[
				$(
					(Self::$kind, $text),
				)*
			];

			fn for_display(&self) -> &'static str {
				match self {
					$(
						Self::$kind => concat!("op ", $text),
					)*

					$(
						Self::$kind_word => concat!("keyword ", $text_word),
					)*

					$(
						Self::$kind_other(..) => $text_other,
					)*

					$(
						Self::$kind_special => concat!("special ", $text_special),
					)*
				}
			}

			fn get_keyword(word: &str) -> Option<Self> {
				match word {
					$(
						$text_word => Some(Self::$kind_word),
					)*
					_ => None
				}
			}

			fn from_op(expected_op: &str) -> Option<&(Self, &str)> {
				Self::TOKEN_OPERATORS.iter()
					.find(|(_, text)|
						expected_op.starts_with(text)
					)
			}
		}

		impl std::fmt::Display for TokenKind {
			fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
				let const_str = match self {
					$(
						Self::$kind => $text,
					)*

					$(
						Self::$kind_word => $text_word,
					)*

					$(
						Self::$kind_other => $text_other,
					)*

					$(
						Self::$kind_special => $text_special,
					)*
				};

				write!(f, "{const_str}")
			}
		}

		impl std::fmt::Display for Token {
			fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
				match self {
					$(
						Self::$kind_other(val) => write!(f, "{} \"{val}\"", $text_other),
					)*

					_ => f.write_str(
						self.for_display()
					),
				}
			}
		}
	};
}

gen_code_for_ops_and_for_display!(
	operators => { // Operators
		Equals 					=> "==",
		Neq 					=> "!=",
		GreatOrEquals 			=> ">=",
		LessOrEquals 			=> "<=",
		Great 					=> ">",
		Less 					=> "<",
		LogicalAnd 				=> "&&",
		LogicalOr 				=> "||",

		Assignment 				=> "=",
		PlusAssignment 			=> "+=",
		MinusAssignment 		=> "-=",
		PowAssignment 			=> "**=",
		MultiplyAssignment 		=> "*=",
		DivideAssignment 		=> "/=",
		RemainderAssignment 	=> "%=",

		Plus 					=> "+",
		Minus 					=> "-",
		Exclamation 			=> "!",
		Pow 					=> "**",
		Multiply 				=> "*",
		Divide 					=> "/",
		Remainder 				=> "%",
		Ref 					=> "&",

		Comma 					=> ",",
		Semicolon 				=> ";",
		LPar 					=> "(",
		RPar 					=> ")",
		LBracket 				=> "[",
		RBracket 				=> "]",
		LBrace 					=> "{",
		RBrace 					=> "}",

		Everything 				=> "...",
		Range 					=> ".."
	}
	keywords => { // Keywords
		True 	=> "true",
		False 	=> "false",
		If 		=> "if",
		Else 	=> "else",
		While 	=> "while",
		For 	=> "for",
		Break 	=> "break",
		Return 	=> "return",
		Global 	=> "global",
		Def 	=> "def"
	}
	literals => { // Literals
		Number(i32) 	=> "number",
		Float(f32) 		=> "float",
		String(String) 	=> "string",
		Word(String) 	=> "word"
	}
	special => { // special
		Eof => "Eof"
	}
);

#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum LexerError {
	#[error("unknown operation")]
	UnknownOperation,
	#[error("empty character literal")]
	EmptyCharacterLiteral,
	#[error("unknown character escape '\\{0}'")]
	UnknownCharacterEscape(char),
	#[error("{0}")]
	InvalidNumber(#[from] ParseIntError),
	#[error("{0}")]
	InvalidFloat(#[from] ParseFloatError),
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

	fn consume(&mut self, expected_c: char) -> Result<(), LexerError> {
		match self.peek(0) {
			Some(c) if c == expected_c => {
				self.consume_next(1);

				Ok(())
			},
			_ => Err(LexerError::ExpectedSymbol { c: expected_c })
		}
	}

	fn tokenize_operator(&mut self) -> Result<Token, LexerError> {
		let expected_op = self.get().take(3).collect::<String>();

		let Some((result, text)) = Token::from_op(&expected_op) else {
		 	return Err(LexerError::UnknownOperation)
		};

		self.consume_next(text.len());

		Ok(result.clone())
	}

	fn tokenize_number(&mut self) -> Result<Token, LexerError> {
		let mut word_chr_cnt = 0;

		let mut dots: usize = 0;

		let mut number_str = String::new();

		for c in self.get() {
			if c == '.' && dots == 0 {
				dots += 1;
			} else if c == '.' || !c.is_ascii_digit() {
				break;
			}

			number_str.push(c);

			word_chr_cnt += 1;
		}

		if let Some('.') = number_str.chars().last() {
			number_str.pop();

			word_chr_cnt -= 1;
		}

		self.consume_next(word_chr_cnt);

		match number_str.parse::<i32>() {
			Ok(result) => {
				Ok(Token::Number(result))
			}
			Err(e) if *e.kind() == IntErrorKind::PosOverflow => {
				let result = number_str.parse()?;

				Ok(Token::Float(result))
			},
			Err(e) => Err(e.into()),
		}
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

		if let Some(token_type) = Token::get_keyword(word_str.as_str()) {
			self.consume_next(word_chr_cnt);

			token_type
		} else {
			self.consume_next(word_chr_cnt);

			Token::Word(word_str)
		}
	}

	fn tokenize_one_char(&mut self, end_sym: char) -> Result<Option<char>, LexerError> {
		let Some(c) = self.peek(0) else {
			return Ok(None)
		};

		if c == end_sym {
			return Ok(None)
		}

		self.consume_next(1);

		if c == '\\' {
			let Some(next) = self.peek(0) else {
				return Ok(None)
			};

			self.consume_next(1);

			let result = match next {
				next if next == end_sym =>
					return Ok(None),
				'n' => '\n',
				'r' => '\r',
				't' => '\t',
				'\'' => '\'',
				'"' => '"',
				// 'x' => '"',
				_ => {
					return Err(LexerError::UnknownCharacterEscape(next))
				}
			};

			Ok(Some(result))
		} else {
			Ok(Some(c))
		}
	}

	fn tokenize_string(&mut self) -> Result<Token, LexerError> {
		self.consume('"')?;

		let mut word_str = String::new();

		while let Some(c) = self.tokenize_one_char('"')? {
			if c == '"' {
				break;
			}

			word_str.push(c);
		}

		self.consume('"')?;

		Ok(Token::String(word_str))
	}

	fn tokenize_char(&mut self) -> Result<Token, LexerError> {
		self.consume('\'')?;

		if let Some(c) = self.tokenize_one_char('\'')? {
			self.consume('\'')?;

			Ok(Token::Number((c as i32) & 0xFF))
		} else {
			Err(
				LexerError::EmptyCharacterLiteral
			)
		}
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

	fn tokenize(&mut self) -> Result<Spanned<Token>, Spanned<LexerError>> {
		let span = self.spans.len();

		loop {
			match self.peek(0) {
				Some(c) if c.is_whitespace() =>
		 			self.skip_spaces(),
				Some('#') =>
					self.skip_comments(),
				Some(_) => break,
				None => return Ok(Spanned::from_with_span(Token::Eof, span))
			}
		}

		let span = self.spans.len();
		self.spans.push((self.row, self.column));

		let result = match self.peek(0) {
			Some(c) if c.is_ascii_alphabetic() || c == '_' =>
				Ok(self.tokenize_word()),
			Some('"') =>
				self.tokenize_string(),
			Some('\'') =>
				self.tokenize_char(),
			Some(c) if c.is_ascii_digit() =>
				self.tokenize_number(),
			Some(_) =>
				self.tokenize_operator(),

			None => Ok(Token::Eof)
		};

		res_with_pos!(result, span)
	}

	/// Парсит текущий код из Lexer на токены
	///
	/// # Пример
	/// ```
	/// let mut spans = Vec::new();
	/// let lexer = Lexer::new(&mut spans, "2 + 2 * 2".chars());
	/// let result = lexer.tokenize_loop()?;
	/// for token in &result {
	///     println!("{token}");
	/// }
	/// ```
	pub fn tokenize_loop(&mut self) -> Result<Vec<Spanned<Token>>, Spanned<LexerError>> {
		let mut result: Vec<Spanned<Token>> = Vec::new();

		while let token = self.tokenize()? && token != Token::Eof {
			result.push(token);
		}

		let span = self.spans.len();
		self.spans.push((self.row, self.column));

		result.push(Spanned::from_with_span(Token::Eof, span));

		Ok(result)
	}
}
