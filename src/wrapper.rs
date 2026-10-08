use crate::{lexer::{LexerErrorRaw, RawToken}};
use std::fmt;

pub trait PosWrapper {
	fn with_pos(self, row: usize, column: usize) -> Self;
	fn get_pos(&self) -> (usize, usize);
}

macro_rules! gen_wrapper_for {
	($type:ident, $wrapper:ident) => {
		pub struct $wrapper {
			row: usize, column: usize,
			val: $type,
		}

		impl std::ops::Deref for $wrapper {
			type Target = $type;

			fn deref(&self) -> &Self::Target {
				&self.val
			}
		}

		impl std::ops::DerefMut for $wrapper {
			fn deref_mut(&mut self) -> &mut Self::Target {
				&mut self.val
			}
		}

		impl std::cmp::PartialEq for $wrapper {
			fn eq(&self, rhs: &Self) -> bool {
				self.val == rhs.val
			}
		}

		impl PosWrapper for $wrapper {
			fn with_pos(mut self, row: usize, column: usize) -> Self {
				self.row = row; self.column = column;

				self
			}

			fn get_pos(&self) -> (usize, usize) {
				(self.row, self.column)
			}
		}

		impl From<$type> for $wrapper {
			fn from(val: $type) -> Self {
				Self {row: 0, column: 0, val}
			}
		}

		impl From<$wrapper> for $type {
			fn from(wrapper: $wrapper) -> Self {
				wrapper.val
			}
		}

		impl fmt::Display for $wrapper {
			fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
				write!(f, "line {} at {}: {}", self.row + 1, self.column + 1, self.val)
			}
		}
	};
}

#[macro_export]
macro_rules! res_with_pos {
	($val: expr, $wrapper_t: ident, $wrapper_e: ident, $row: expr, $column: expr) => {
		$wrapper_t::from($val.map_err(|e| $wrapper_e::from(e).with_pos($row, $column))?).with_pos($row, $column)
	};
}

#[macro_export]
macro_rules! val_with_pos {
	($val: expr, $wrapper_t: ident, $row: expr, $column: expr) => {
		$wrapper_t::from($val).with_pos($row, $column)
	};
}

gen_wrapper_for!(LexerErrorRaw, LexerError);
// gen_wrapper_for!(RawParserError, ParserError);
gen_wrapper_for!(RawToken, Token);
