use std::fmt::Write;
use crate::{lexer::{LexerErrorRaw, RawToken}, optimizer::RawOptimizerError, parser::{RawASTNode, RawParserError}, uni_type::{RawUniResult, RawUniResultError}};
use std::fmt;

pub trait PosWrapper {
	fn get_pos(&self) -> usize;
}

macro_rules! gen_wrapper_for {
	($type:ident, $wrapper:ident) => {
		#[derive(Debug, Clone)]
		pub struct $wrapper {
			pub span: usize,
			pub val: $type,
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

		impl std::cmp::PartialEq<$type> for $wrapper {
			fn eq(&self, rhs: &$type) -> bool {
				self.val == *rhs
			}
		}

		impl PosWrapper for $wrapper {
			fn get_pos(&self) -> usize {
				self.span
			}
		}

		impl $wrapper {
			pub fn with_pos(mut self, span: usize) -> Self {
				self.span = span;

				self
			}
			pub fn with_pos_from<W>(mut self, from: &W) -> Self where W: PosWrapper {
				self.span = from.get_pos();

				self
			}

			pub fn format(&self, spans: &[(usize, usize)], f: &mut String) -> fmt::Result {
				let (row, column) = spans.get(self.span).unwrap();

				write!(f, "line {} at {}: {}", row + 1, column + 1, self.val)
			}
		}

		impl From<$type> for $wrapper {
			fn from(val: $type) -> Self {
				Self {span: 0, val}
			}
		}

		impl From<$wrapper> for $type {
			fn from(wrapper: $wrapper) -> Self {
				wrapper.val
			}
		}

		impl fmt::Display for $wrapper {
			fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
				write!(f, "{}", self.val)
			}
		}
	};
}

#[macro_export]
macro_rules! val_with_pos {
	($val: expr, $wrapper: ident, $span: expr) => {
		$wrapper::from($val).with_pos($span)
	};
}

#[macro_export]
macro_rules! res_with_pos {
	($val: expr, $wrapper_t: ident, $wrapper_e: ident, $span: expr) => {
		$val.map_or_else(
			|e| Err($wrapper_e::from(e).with_pos($span)),
			|v| Ok($wrapper_t::from(v).with_pos($span))
		)
	};
}

#[macro_export]
macro_rules! wmatch {
	($val:pat) => {
		Token { span: _, val: $val }
	};
}

gen_wrapper_for!(LexerErrorRaw, LexerError);
gen_wrapper_for!(RawParserError, ParserError);
gen_wrapper_for!(RawOptimizerError, OptimizerError);
gen_wrapper_for!(RawUniResultError, UniResultError);

gen_wrapper_for!(RawToken, Token);
gen_wrapper_for!(RawASTNode, ASTNode);
gen_wrapper_for!(RawUniResult, UniResult);

impl Default for ASTNode {
	fn default() -> Self {
		Self { span: 0, val: RawASTNode::default() }
	}
}
