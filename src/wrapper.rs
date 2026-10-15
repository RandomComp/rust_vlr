use std::fmt;

use crate::compiler::optimizer::OptimizerError;
use crate::compiler::parser::ASTNode;
use crate::uni_type::UniResultError;
use crate::vm::RawVMError;

type Span = usize;

#[derive(Debug, Clone)]
pub struct Spanned<T> {
	pub val: T,
	pub span: Span
}

impl<T: fmt::Display> Spanned<T> {
	pub fn from_with_span(val: T, span: Span) -> Self {
		Self {val, span}
	}

	pub fn format(&self, spans: &[(usize, usize)]) -> String {
		let (row, column) = spans.get(self.span).unwrap_or_else(|| panic!("invalid span present {}", self.span));

		format!("line {} at {}: {}", row + 1, column + 1, self.val)
	}
}

impl<T> std::ops::Deref for Spanned<T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		&self.val
	}
}

impl<T> std::ops::DerefMut for Spanned<T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.val
	}
}

impl<T: PartialEq> PartialEq for Spanned<T> {
	fn eq(&self, rhs: &Self) -> bool {
		self.val == rhs.val
	}
}

impl<T: PartialEq> PartialEq<T> for Spanned<T> {
	fn eq(&self, rhs: &T) -> bool {
		self.val == *rhs
	}
}

// impl From<Spanned<T>> for $type {
// 	fn from(wrapper: $wrapper) -> Self {
// 		wrapper.val
// 	}
// }

impl<T: fmt::Display> fmt::Display for Spanned<T> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}", self.val)
	}
}

#[macro_export]
macro_rules! val_with_pos {
	($val: expr, $wrapper: ident, $span: expr) => {
		$wrapper::from_with_span($val, $span)
	};
}

#[macro_export]
macro_rules! res_with_pos {
	($val: expr, $span: expr) => {
		$val.map_or_else(
			|e| Err(Spanned::from_with_span(e, $span)),
			|v| Ok(Spanned::from_with_span(v, $span))
		)
	};
}

#[macro_export]
macro_rules! wmatch {
	($val:pat) => {
		Spanned { span: _, val: $val }
	};
}

macro_rules! wrapper_connect {
	( $( $from:ident => $to:ident ),* $(,)? ) => {
  		$(
	  		impl From<Spanned<$from>> for Spanned<$to> {
				fn from(from: Spanned<$from>) -> Spanned<$to> {
					let span = from.span;

					Spanned::from_with_span(
						<$to>::from(from.val),
						span
					)
				}
			}
    	)*
    };
}

wrapper_connect!(
	UniResultError => OptimizerError,
	UniResultError => RawVMError,
);

impl Default for Spanned<ASTNode> {
	fn default() -> Self {
		Self { span: 0, val: ASTNode::default() }
	}
}
