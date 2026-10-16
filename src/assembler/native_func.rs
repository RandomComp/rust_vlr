use std::{cmp::Ordering, fmt::Display};

use crate::{assembler::VMWord, vm::Memory};

#[derive(thiserror::Error, Clone, Debug)]
pub enum NativeFunctionIDError {
	#[error("Unknown native function with id {0}")]
	UnknownNativeFunctionWithID(u8),
	#[error("Unknown native function {0}")]
	UnknownNativeFunctionWithName(String),

	#[error("Too few arguments for native function '{id}' call, expected {expected}, found {found}")]
	ArgumentsNativeFunctionCall {id: NativeFunctionID, found: u32, expected: u32 },

	#[error("exit code: {0}")]
	Exit(u32),
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

			fn print(args: &[VMWord]) -> Result<VMWord, NativeFunctionIDError> {
				for arg in args {
					print!("{arg} ");
				}

				println!();

				Ok(0)
			}
			fn exit(args: &[VMWord]) -> Result<VMWord, NativeFunctionIDError> {
				if args.len() != 1 {
					unreachable!("invalid native function exit call");
				}

				Err(NativeFunctionIDError::Exit(args[0].clone()))
			}

			pub fn call(self, mem: &mut Memory) -> Result<VMWord, NativeFunctionIDError> {
				let mut args = Vec::new();

				let expected = self.args();

				let mut i = 0;

				match expected {
					NativeFunctionArgsNumber::Fixed(fixed) => {
						while i < fixed  {
							let value = mem.pop().map_err(|_|
								NativeFunctionIDError::ArgumentsNativeFunctionCall {
									id: self,
									found: i,
									expected: fixed
								}
							)?;

							args.push(value);

							i += 1;
						}
					}
					NativeFunctionArgsNumber::Dynamic => {
						if let Ok(fixed) = mem.pop() {
							while i < fixed  {
								let Ok(value) = mem.pop() else {
									return Err(
										NativeFunctionIDError::ArgumentsNativeFunctionCall {
											id: self,
											found: i,
											expected: fixed
										}
									)
								};

								args.push(value);

								i += 1;
							}
						}
					}
				}

				args.reverse();

				match self {
					NativeFunctionID::Print =>
						Ok(Self::print(&args)?),
					NativeFunctionID::Exit =>
						Self::exit(&args),
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
