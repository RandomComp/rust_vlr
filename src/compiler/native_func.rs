use std::fmt::Display;

use crate::vm::{Memory, VMWord};

#[derive(thiserror::Error, Clone, Debug)]
pub enum NativeFunctionIDError {
	#[error("Unknown native function with id {0}")]
	UnknownNativeFunctionWithID(u8),
	#[error("Unknown native function {0}")]
	UnknownNativeFunctionWithName(String),

	#[error("Expected number of dynamic arguments in stack")]
	ExpectedDynamicArgsNumber,
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

macro_rules! from_args_type {
	( dynamic: $args:expr ) => {
		NativeFunctionArgsNumber::Dynamic
	};
	( fixed: $args:expr ) => {
		NativeFunctionArgsNumber::Fixed($args)
	};
}

macro_rules! trash {
	( $($_:tt )*  ) => {
		0
	};
}

macro_rules! args_cnt {
	( $($arg:ident ),* ) => {
		0 $( + 1 + trash!($arg))*
	};
}

macro_rules! call_implementation {
	( $mem_ptr:expr, $impl: block, $args:ident, $mem:ident, dynamic: $arg:ident ) => {
		{
			let $arg = &$args;

			let $mem = $mem_ptr;

			#[warn(unused_assignments)]
			$impl
		}
	};
	( $mem_ptr:expr, $impl: block, $args:ident, $mem:ident, fixed: $( $arg:ident ),* ) => {
		{
			let mut index = 0;

			$(
				let $arg = $args[index];

				index += 1;
			)*

			#[allow(unused_variables)]
			let $mem = $mem_ptr;

			#[warn(unused_assignments)]
			$impl
		}
	};
}

macro_rules! gen_native_functions {
	(
		$(
			$name:ident( $mem:ident, $args_type:ident: $( $arg:ident ),*  ) = $id:literal as $text:literal => $implementation:block
		),*
		$(,)?
	) => {
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
						Self::$name => from_args_type!(
							$args_type: args_cnt!( $( $arg ),* )
						),
					)*
				}
			}

			pub fn call(self, mem: &mut Memory) -> Result<VMWord, NativeFunctionIDError> {
				let mut args = Vec::new();

				let expected = self.args();

				let mut i = 0;

				match expected {
					NativeFunctionArgsNumber::Fixed(fixed) => {
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
					NativeFunctionArgsNumber::Dynamic => {
						let Ok(fixed) = mem.pop() else {
							return Err(NativeFunctionIDError::ExpectedDynamicArgsNumber)
						};

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

				args.reverse();

				match self {
					$(
						#[allow(unused_assignments)]
						NativeFunctionID::$name =>
							call_implementation!(mem, $implementation, args, $mem, $args_type: $( $arg ),* ),
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
	Write(mem, fixed: fd, addr, cnt) = 0 as "write" => {
		// println!("fd = {fd}, addr = {addr}, cnt = {cnt}");

		if fd == 1 {
			let mut buf = Vec::new();

			for byte in mem.mem_iter(addr as usize).take(cnt as usize) {
				buf.push(byte);
			}

			print!("{}", String::from_utf8_lossy_owned(buf));
		}

		Ok(0)
	},
	Exit(mem, fixed: code) = 1 as "exit" => {
		Err(NativeFunctionIDError::Exit(code))
	},
);
