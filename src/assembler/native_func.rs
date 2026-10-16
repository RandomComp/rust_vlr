use std::{cmp::Ordering, fmt::Display};

#[derive(thiserror::Error, Clone, Debug)]
pub enum NativeFunctionIDError {
	#[error("Unknown native function with id {0}")]
	UnknownNativeFunctionWithID(u8),
	#[error("Unknown native function {0}")]
	UnknownNativeFunctionWithName(String),
}

#[derive(Clone, Debug)]
pub enum NativeFunctionArgsNumber {
	Fixed(u32),
	Dynamic,
}

impl PartialEq<u32> for NativeFunctionArgsNumber {
	fn eq(&self, other: &u32) -> bool {
		match self {
			NativeFunctionArgsNumber::Fixed(fixed) =>
				fixed == other,
			NativeFunctionArgsNumber::Dynamic => false,
		}
	}
}

impl PartialOrd<u32> for NativeFunctionArgsNumber {
	fn partial_cmp(&self, other: &u32) -> Option<std::cmp::Ordering> {
		match self {
			NativeFunctionArgsNumber::Fixed(fixed) => {
				Some(fixed.cmp(other))
			},
			NativeFunctionArgsNumber::Dynamic => Some(Ordering::Greater),
		}
	}
}

impl Display for NativeFunctionArgsNumber {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			NativeFunctionArgsNumber::Fixed(fixed) =>
				write!(f, "{fixed}"),
			NativeFunctionArgsNumber::Dynamic =>
				write!(f, "dynamic"),
		}
	}
}

macro_rules! gen_native_functions {
	( $( $text:expr => $name:ident($args_cnt:expr) = $id:expr ),* $(,)? ) => {
		#[derive(PartialEq, Eq, Debug, Copy, Clone)]
		pub enum NativeFunctionID {
			$(
				$name,
			)*
		}

		impl NativeFunctionID {
			pub fn to_le_bytes(self) -> [u8; 1] {
				(self as u8).to_le_bytes()
			}

			pub fn args(self) -> NativeFunctionArgsNumber {
				match self {
					$(
						Self::$name => $args_cnt,
					)*
				}
			}
		}

		impl Display for NativeFunctionID {
			fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
				match self {
					$(
						Self::$name => write!(f, $text),
					)*
				}
			}
		}

		impl TryFrom<u8> for NativeFunctionID {
			type Error = NativeFunctionIDError;

			fn try_from(value: u8) -> Result<Self, Self::Error> {
				match value {
					$(
						$id => Ok(Self::$name),
					)*
					_ => Err(NativeFunctionIDError::UnknownNativeFunctionWithID(value)),
				}
			}
		}

		impl TryFrom<&str> for NativeFunctionID {
			type Error = NativeFunctionIDError;

			fn try_from(name: &str) -> Result<Self, Self::Error> {
				match name {
					$(
						$text => Ok(Self::$name),
					)*
					_ => Err(
						NativeFunctionIDError::UnknownNativeFunctionWithName(
							name.to_string()
						)
					),
				}
			}
		}
	};
}

gen_native_functions!(
	"print" => Print(NativeFunctionArgsNumber::Dynamic) 	= 0,
	"exit" => Exit(NativeFunctionArgsNumber::Fixed(1)) 		= 1,
);
