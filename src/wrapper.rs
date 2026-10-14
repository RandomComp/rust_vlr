use std::fmt::Write;
use std::fmt;

use crate::compiler::parser::ASTNode;

type Span = usize;

#[derive(Debug, Clone)]
pub struct Spanned<T> {
	pub val: T,
	pub span: Span
}

impl<T: fmt::Display> Spanned<T> {
	pub fn get_span(&self) -> Span {
		self.span
	}

	pub fn from_with_span(val: T, span: Span) -> Self {
		Self {span, val}
	}

	pub fn format(&self, spans: &[(usize, usize)], f: &mut String) -> fmt::Result {
		let (row, column) = spans.get(self.span).unwrap_or_else(|| panic!("invalid span present {}", self.span));

		write!(f, "line {} at {}: {}", row + 1, column + 1, self.val)
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

// map_err(|e| OptimizerError::from(RawOptimizerError::from(e.val))

// macro_rules! wrapper_connect {
// 	( $( $from_wrapped:ident ($from_raw:ty) => $to_wrapped:ident ($to_raw:ty) ),* $(,)* ) => {
//   		$(
// 	  		impl From<$from_wrapped> for $to_wrapped {
// 				fn from(from: $from_wrapped) -> $to_wrapped {
// 					let span = from.span;
//
// 					<$to_wrapped>::from_with_span(
// 						<$to_raw>::from(from.val),
// 						span
// 					)
// 				}
// 			}
//
// 	 	// 	impl From<$from_raw> for $to_wrapped {
// 			// 	fn from(from: $from_raw) -> $to_wrapped {
// 			// 		<$to_wrapped>::from(
// 			// 			<$to_raw>::from(from)
// 			// 		)
// 			// 	}
// 			// }
//     	)*
//     };
// }
//
// wrapper_connect!(
// 	UniResultError(RawUniResultError) => OptimizerError(RawOptimizerError),
// );

impl Default for Spanned<ASTNode> {
	fn default() -> Self {
		Self { span: 0, val: ASTNode::default() }
	}
}
