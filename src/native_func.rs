use std::{cmp::Ordering, fmt::Display};

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
#[repr(u8)]
pub enum NativeFunctionID {
	Print,
	Tuple,
	Range
}

#[derive(thiserror::Error, Clone, Debug)]
pub enum NativeFunctionIDError {
	#[error("Unknown native function with id {0}")]
	UnknownNativeFunctionWithID(u8),
	#[error("Unknown native function {0}")]
	UnknownNativeFunctionWithName(String),
}

impl Display for NativeFunctionID {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			NativeFunctionID::Print => write!(f, "print"),
			NativeFunctionID::Tuple => write!(f, "tuple"),
			NativeFunctionID::Range => write!(f, "range"),
		}
	}
}

#[derive(Clone, Debug)]
pub enum NativeFunctionArgsNumber {
	Fixed(usize),
	Dynamic,
}

impl PartialEq<usize> for NativeFunctionArgsNumber {
	fn eq(&self, other: &usize) -> bool {
		match self {
			NativeFunctionArgsNumber::Fixed(fixed) =>
				fixed == other,
			NativeFunctionArgsNumber::Dynamic => false,
		}
	}
}

impl PartialOrd<usize> for NativeFunctionArgsNumber {
	fn partial_cmp(&self, other: &usize) -> Option<std::cmp::Ordering> {
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

impl NativeFunctionID {
	pub fn to_le_bytes(self) -> [u8; 1] {
		(self as u8).to_le_bytes()
	}

	pub fn get_name(self) -> &'static str {
		match self {
			NativeFunctionID::Print => "print",
			NativeFunctionID::Tuple => "tuple",
			NativeFunctionID::Range => "range",
		}
	}

	pub fn args(self) -> NativeFunctionArgsNumber {
		match self {
			NativeFunctionID::Print => NativeFunctionArgsNumber::Dynamic,
			NativeFunctionID::Tuple => NativeFunctionArgsNumber::Dynamic,
			NativeFunctionID::Range => NativeFunctionArgsNumber::Fixed(3),
		}
	}
}

impl TryFrom<u8> for NativeFunctionID {
	type Error = NativeFunctionIDError;

	fn try_from(value: u8) -> Result<Self, Self::Error> {
		match value {
			0 => Ok(NativeFunctionID::Print),
			1 => Ok(NativeFunctionID::Tuple),
			2 => Ok(NativeFunctionID::Range),
			_ => Err(NativeFunctionIDError::UnknownNativeFunctionWithID(value)),
		}
	}
}

impl TryFrom<&str> for NativeFunctionID {
	type Error = NativeFunctionIDError;

	fn try_from(name: &str) -> Result<Self, Self::Error> {
		match name {
			"print" => Ok(NativeFunctionID::Print),
			"tuple" => Ok(NativeFunctionID::Tuple),
			"range" => Ok(NativeFunctionID::Range),
			_ => Err(
				NativeFunctionIDError::UnknownNativeFunctionWithName(
					name.to_owned()
				)
			),
		}
	}
}
