use std::string::FromUtf8Error;

use crate::compiler::native_func::{NativeFunctionIDError};

#[derive(thiserror::Error, Clone, Debug)]
pub enum BytecodeError {
	#[error("Byte {byte:02X} at {pos:02} unexpected")]
	UnexpectedByte {
		pos: u32, byte: u8
	},
	#[error("Unexpected EOF at {pos}")]
	UnexpectedEof {
		pos: u32,
	},
	#[error("String constant length is too long for u32 type, that means string is bigger than 4 GiB, try using dynamic string")]
	StringTooLong,
	#[error("String have invalid UTF-8 invalid sequence: ")]
	StringHasInvalidUTF8Sequence(#[from] FromUtf8Error),
	#[error("Invalid register 0x{0:02X}")]
	InvalidRegisterByte(u8),
	#[error("{0}")]
	NativeFunctionID(#[from] NativeFunctionIDError),
}

#[macro_export]
macro_rules! gen_regs_code {
	(
		$(
			$( #[$doc:meta] )*
			$byte:literal | $enum_name:ident => $name:ident
		),+
		$(,)?
	) => {
		use $crate::vm::VMWord;

		#[derive(PartialEq, Copy, Clone, Debug)]
		pub enum BytecodeRegister {
			$(
				$( #[$doc] )*
				$enum_name,
			)+
		}

		#[derive(Default, Debug)]
		pub struct VMRegs {
			$(
				$( #[$doc] )*
				pub $name: VMWord,
			)+
		}

		impl std::ops::Index<BytecodeRegister> for VMRegs {
			type Output = VMWord;

			fn index(&self, index: BytecodeRegister) -> &Self::Output {
				match index {
					$(
						BytecodeRegister::$enum_name => &self.$name,
					)+
				}
			}
		}

		impl std::ops::IndexMut<BytecodeRegister> for VMRegs {
			fn index_mut(&mut self, index: BytecodeRegister) -> &mut Self::Output {
				match index {
					$(
						BytecodeRegister::$enum_name => &mut self.$name,
					)+
				}
			}
		}

		impl fmt::Display for VMRegs {
			fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
				$(
					write!(f, "{} = {}; ", BytecodeRegister::$enum_name, self.$name)?;
				)+

				Ok(())
			}
		}

		impl fmt::Display for BytecodeRegister {
			fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
				match self {
					$(
						Self::$enum_name => stringify!($name),
					)+
				}.fmt(f)
			}
		}

		impl BytecodeRegister {
			pub fn to_le_bytes(self) -> [u8; 1] {
				(self as u8).to_le_bytes()
			}
		}

		impl TryFrom<BytecodeRegister> for u8 {
			type Error = BytecodeError;

			fn try_from(value: BytecodeRegister) -> Result<Self, Self::Error> {
				match value {
					$(
						BytecodeRegister::$enum_name => Ok($byte),
					)+
				}
			}
		}

		impl TryFrom<u8> for BytecodeRegister {
			type Error = BytecodeError;

			fn try_from(value: u8) -> Result<Self, Self::Error> {
				match value {
					$(
						$byte => Ok(Self::$enum_name),
					)+
					_ => Err(BytecodeError::InvalidRegisterByte(value))
				}
			}
		}
	};
}

#[macro_export]
macro_rules! gen_insts_impl {
	(
		$(
			$( ( bin $binary_op:ident ) )? $( ( un $unary_op:ident ) )?
			$inst:ident $( ( $($arg:ty),* $(,)? ) )? as $main_opcode:literal $( | $( $opcode:literal )|* )?
			=>
			{
				asm 	( $result:ident $(, $($asm_arg:ident),* $(,)? )? ) => $inst_asm:block,
				disasm ( $bytes:ident ) => $inst_disasm:block,
				exec 	( $mem:ident $(, $( $exec_arg:ident ),* $(,)? )? ) => $inst_exec:block,
				display ( $formatter:ident $(, $($display_arg:ident),* $(,)? )? ) => $inst_display:block
			}
		),*
		$(,)?
	) => {
		use $crate::vm::{Memory, RawVMError};
		use $crate::compiler::parser::{UnaryOp, BinaryOp};

		#[derive(PartialEq, Clone, Debug)]
		pub enum Bytecode {
			$(
				$inst $( ( $( $arg, )* ) )?,
			)*
		}

		impl fmt::Display for Bytecode {
			fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
				match self {
					$(
						Self::$inst $( ( $( $display_arg, )* ) )? => {
							let $formatter = f;

							$inst_display
						}
					)*
				}
			}
		}

		impl Bytecode {
			fn asm_inst(&self) -> Result<Vec<u8>, BytecodeError> {
				let mut result = Vec::new();

				match self {
					$(
						Self::$inst $( ( $( $asm_arg, )* ) )? => {
							result.push($main_opcode);

							let mut $result = result;

							$inst_asm;

							Ok($result)
						},
					)*
				}
			}

			pub fn asm(values: Vec<Self>) -> Result<Vec<u8>, BytecodeError> {
				let mut result = Vec::new();

				for value in values {
					let res: Vec<u8> = value.asm_inst()?;

					result.extend(res);
				}

				Ok(result)
			}

			pub fn disasm_inst<T>(mut bytes: T) -> Result<Option<Self>, BytecodeError>
				where T: Iterator<Item = u8> {
				let Some(opcode) = bytes.next() else {
					return Ok(None)
				};

				let result = match opcode {
					$(
						$main_opcode $( | $( $opcode )|* )? => {
							let mut $bytes = bytes;

							$inst_disasm
						},
					)*
					x => return Err(BytecodeError::UnexpectedByte { pos: 0, byte: x }), // TODO: fix: pos: 0 in bytecode disasembler
				};

				Ok(Some(result))
			}

			pub fn disasm<T>(mut bytes: T) -> Result<Vec<Self>, BytecodeError> where T: Iterator<Item = u8> {
				let mut result = Vec::new();

				while let Some(val) = Self::disasm_inst(&mut bytes)? {
					result.push(val);
				}

				Ok(result)
			}

			pub fn exec(&self, mem: &mut Memory) -> Result<(), RawVMError> {
				match self {
					$(
						&Self::$inst $( ( $( $exec_arg, )* ) )? => {
							let mut $mem = mem;

							$inst_exec;

							Ok(())

							// result
						},
					)*
				}
			}

			pub fn from_unary_op(op: &UnaryOp) -> Bytecode {
				match op {
					$(
						$(
							UnaryOp::$unary_op => Self::$inst,
						)?
					)*
				}
			}

			pub fn from_binary_op(op: &BinaryOp) -> Bytecode {
				match op {
					$(
						$(
							BinaryOp::$binary_op => Self::$inst,
						)?
					)*
					_ => unreachable!("'{op}' not handled in from_binary_op"),
				}
			}
		}
	};
}
