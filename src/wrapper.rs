use std::fmt::Write;
use crate::{compiler::RawCompilerError, compiler::lexer::{RawLexerError, RawToken}, compiler::optimizer::RawOptimizerError, compiler::parser::{RawASTNode, RawParserError}, uni_type::{RawUniResult, RawUniResultError}};
use std::fmt;

pub trait PosWrapper {
	fn get_pos(&self) -> usize;
}

macro_rules! gen_wrapper_for {
	($type:ty, $wrapper:ident) => {
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
				let (row, column) = spans.get(self.span).expect(&format!("invalid span present {}", self.span));

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

macro_rules! gen_wrappers_for {
	( $($raw:ty => $wrapped:ident),* $(,)*) => {
		$(
			gen_wrapper_for!($raw, $wrapped);
		)*
	};
}

// map_err(|e| OptimizerError::from(RawOptimizerError::from(e.val))

macro_rules! wrapper_connect {
	( $( $from_wrapped:ident ($from_raw:ty) => $to_wrapped:ident ($to_raw:ty) ),* $(,)* ) => {
  		$(
	  		impl From<$from_wrapped> for $to_wrapped {
				fn from(from: $from_wrapped) -> $to_wrapped {
					let span = from.span;

					<$to_wrapped>::from(
						<$to_raw>::from(from.val)
					).with_pos(span)
				}
			}

	 		impl From<$from_raw> for $to_wrapped {
				fn from(from: $from_raw) -> $to_wrapped {
					<$to_wrapped>::from(
						<$to_raw>::from(from)
					)
				}
			}
    	)*
    };
}

gen_wrappers_for!(
	RawLexerError 		=> LexerError,
	RawParserError 		=> ParserError,
	RawOptimizerError 	=> OptimizerError,
	RawCompilerError 	=> CompilerError,
	RawUniResultError 	=> UniResultError,

	RawToken 			=> Token,
	RawASTNode 			=> ASTNode,
	RawUniResult 		=> UniResult,
);

wrapper_connect!(
	UniResultError(RawUniResultError) => OptimizerError(RawOptimizerError),
);

impl Default for ASTNode {
	fn default() -> Self {
		Self { span: 0, val: RawASTNode::default() }
	}
}
