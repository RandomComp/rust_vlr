use std::fmt::Display;

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

	pub fn args(self) -> usize {
		match self {
			NativeFunctionID::Print => 1,
			NativeFunctionID::Tuple => 2,
			NativeFunctionID::Range => 3,
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
