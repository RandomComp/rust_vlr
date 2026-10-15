use std::{fmt, string::FromUtf8Error};

use crate::{native_func::{NativeFunctionID, NativeFunctionIDError}, compiler::parser::{BinaryOp, UnaryOp}};

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum BytecodeRegister {
	/// Subprogram result register
	Accum,
	Base,
	BasePointer,
	StackPointer,
}

impl fmt::Display for BytecodeRegister {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Accum => "acc",
			Self::Base => "base",
			Self::BasePointer => "bp",
			Self::StackPointer => "sp",
		}.fmt(f)
	}
}

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
	#[error("{0}")]
	NativeFunctionID(#[from] NativeFunctionIDError),
}

fn consume_const_bytes_and_get<T, const LEN: usize>(arr: &mut T) -> Result<[u8; LEN], BytecodeError> where T: Iterator<Item=u8> {
	let mut result = [0u8; LEN];

	for val in &mut result {
		*val = arr.next().ok_or(BytecodeError::UnexpectedEof { pos: 0 })?;
	}

	Ok(result)
}

macro_rules! gen_asm_disasm {
	(
		$(
			$( ( $modif:ident $binary_op:ident ) )? $inst:ident $( ( $($arg:ty),* $(,)? ) )? as $main_opcode:literal $( | $( $opcode:literal )|* )?
			=>
			{
				asm 	$( ( $($asm_arg:ident),* $(,)? ) )? => $inst_asm:block,
				disasm ( $bytes:ident ) => $inst_disasm:block,
				display ($formatter:ident $(, $($display_arg:ident),* $(,)? )? ) => $inst_display:block
			}
		),*
		$(,)?
	) => {
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
			fn get_opcode(&self) -> u8 {
				match self {
					$(
						Self::$inst $( ( $( $asm_arg, )* ) )? => $main_opcode,
					)*
				}
			}

			fn asm_inst(value: Self) -> Result<Vec<u8>, BytecodeError> {
				let mut result = Vec::new();

				result.push(value.get_opcode());

				match value {
					$(
						Self::$inst $( ( $( $asm_arg, )* ) )? => $inst_asm,
					)*
				}

				Ok(result)
			}

			pub fn asm(values: Vec<Self>) -> Result<Vec<u8>, BytecodeError> {
				let mut result = Vec::new();

				for value in values {
					let res: Vec<u8> = Self::asm_inst(value)?;

					result.extend(res);
				}

				Ok(result)
			}

			pub fn disasm_inst<T>(mut bytes: T) -> Result<Option<Self>, BytecodeError> where T: Iterator<Item = u8> {
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
			pub fn to_binary_op(&self) -> Option<BinaryOp> {
				match self {
					$(
						$(
							Self::$inst => Some(BinaryOp::$binary_op),
						)?
					)*
					_ => None,
				}
			}
		}
	};
}

gen_asm_disasm!(
	Halt as 0x00 | 0xFF => {
		asm => {

		},
		disasm(bytes) => {
			Self::Halt
		},
		display(f) => {
			write!(f, "hlt")
		}
	},
	PushNumber(u32) as 0x01 => {
		asm(val) => {

		},
		disasm(bytes) => {
			let value = u32::from_le_bytes(consume_const_bytes_and_get::<T, 4>(&mut bytes)?);

			Self::PushNumber(value)
		},
		display(f, val) => {
			write!(f, "")
		}
	},
	PushFloat(f32) as 0x02 => {
		asm(val) => {

		},
		disasm(bytes) => {
			Self::Halt
		},
		display(f, val) => {
			write!(f, "")
		}
	},
	PushString(String) as 0x03 => {
		asm(val) => {

		},
		disasm(bytes) => {
			Self::Halt
		},
		display(f, val) => {
			write!(f, "")
		}
	},

	PushReg(BytecodeRegister) as 0x04 => {
		asm(reg) => {

		},
		disasm(bytes) => {
			Self::Halt
		},
		display(f, reg) => {
			write!(f, "")
		}
	},
	PopReg(BytecodeRegister) as 0x05 => {
		asm(reg) => {

		},
		disasm(bytes) => {
			Self::Halt
		},
		display(f, reg) => {
			write!(f, "")
		}
	},

	Look(u32) as 0x06 => {
		asm(stack) => {

		},
		disasm(bytes) => {
			Self::Halt
		},
		display(f, stack) => {
			write!(f, "")
		}
	},
	Load(u32) as 0x07 => {
		asm(stack) => {

		},
		disasm(bytes) => {
			Self::Halt
		},
		display(f, stack) => {
			write!(f, "")
		}
	},
	Remove as 0x08 => {
		asm => {

		},
		disasm(bytes) => {
			Self::Remove
		},
		display(f) => {
			write!(f, "")
		}
	},

	MovU32ToReg(BytecodeRegister, u32) as 0x0A => {
		asm(reg, val) => {

		},
		disasm(bytes) => {
			Self::Halt
		},
		display(f, reg, val) => {
			write!(f, "")
		}
	},
	MovRegToReg(BytecodeRegister, BytecodeRegister) as 0x0B => {
		asm(reg1, reg2) => {

		},
		disasm(bytes) => {
			Self::Halt
		},
		display(f, reg1, reg2) => {
			write!(f, "")
		}
	},

	Jif(u32) as 0x20 => {
		asm(id) => {

		},
		disasm(bytes) => {
			Self::Halt
		},
		display(f, id) => {
			write!(f, "")
		}
	},
	Jmp(u32) as 0x28 => {
		asm(id) => {

		},
		disasm(bytes) => {
			Self::Halt
		},
		display(f, id) => {
			write!(f, "")
		}
	},
	Call(u32) as 0x2A => {
		asm(id) => {

		},
		disasm(bytes) => {
			Self::Halt
		},
		display(f, id) => {
			write!(f, "")
		}
	},
	Ret as 0x2C => {
		asm => {

		},
		disasm(bytes) => {
			Self::Halt
		},
		display(f) => {
			write!(f, "")
		}
	},
	Eq as 0x50 => {
		asm => {

		},
		disasm(bytes) => {
			Self::Halt
		},
		display(f) => {
			write!(f, "")
		}
	},
	Lt as 0x52 => {
		asm => {

		},
		disasm(_bytes) => {
			Self::Lt
		},
		display(f) => {
			write!(f, "lt")
		}
	},
	NativeCall(NativeFunctionID) as 0x80 => {
		asm(id) => {

		},
		disasm(bytes) => {
			Self::Halt
		},
		display(f, id) => {
			write!(f, "")
		}
	},
	(bin Plus) Add as 0xA0 => {
		asm => {

		},
		disasm(bytes) => {
			Self::Add
		},
		display(f) => {
			write!(f, "")
		}
	},
	Sub as 0xA3 => {
		asm => {

		},
		disasm(bytes) => {
			Self::Sub
		},
		display(f) => {
			write!(f, "")
		}
	},
	Mul as 0xB0 => {
		asm => {

		},
		disasm(bytes) => {
			Self::Mul
		},
		display(f) => {
			write!(f, "")
		}
	},
	Div as 0xB3 => {
		asm => {

		},
		disasm(bytes) => {
			Self::Div
		},
		display(f) => {
			write!(f, "")
		}
	},
	Pow as 0xC0 => {
		asm => {

		},
		disasm(bytes) => {
			Self::Pow
		},
		display(f) => {
			write!(f, "")
		}
	},
	Neg as 0xC3 => {
		asm => {

		},
		disasm(bytes) => {
			Self::Neg
		},
		display(f) => {
			write!(f, "")
		}
	},
	Not as 0xD0 => {
		asm => {

		},
		disasm(bytes) => {
			Self::Not
		},
		display(f) => {
			write!(f, "")
		}
	},
	LogicalAnd as 0xD3 => {
		asm => {

		},
		disasm(bytes) => {
			Self::Halt
		},
		display(f) => {
			write!(f, "")
		}
	},
	LogicalOr as 0xE0 => {
		asm => {

		},
		disasm(bytes) => {
			Self::Halt
		},
		display(f) => {
			write!(f, "")
		}
	},
);

impl Bytecode {
	pub fn to_unary_op(&self) -> Option<UnaryOp> {
		let result = match self {
			Self::Neg => UnaryOp::Minus, // Negative value (-)
			Self::Not => UnaryOp::Not, // Logical not
			_ => return None,
		};

		Some(result)
	}

	// TODO: use macro_rules! for automatically generate functions for assembling, disassembling and representation


	pub fn from_unary_op(op: &UnaryOp) -> Bytecode {
		match op {
			UnaryOp::Minus => Self::Neg,
			UnaryOp::Not => Self::Not,
		}
	}

	pub fn from_binary_op(op: &BinaryOp) -> Bytecode {
		match op {
			BinaryOp::Plus => Self::Add,
			BinaryOp::Minus => Self::Sub,
			BinaryOp::Multiply => Self::Mul,
			BinaryOp::Divide => Self::Div,
			// BinaryOp::Remainder => Self::Rem,
			BinaryOp::Pow => Self::Pow,
			BinaryOp::Equals => Self::Eq,
			BinaryOp::Less => Self::Lt,
			BinaryOp::LogicalAnd => Self::LogicalAnd,
			BinaryOp::LogicalOr => Self::LogicalOr,
			_ => unreachable!("'{op}' not handled in from_binary_op"),
		}
	}

	pub fn bytecodes_from_binary_op(op: &BinaryOp) -> Vec<Bytecode> {
		let mut result = Vec::new();

		match op {
			BinaryOp::Neq => {
				result.push(Self::Eq);
				result.push(Self::Not);
			}
			BinaryOp::GreatOrEquals => {
				result.push(Self::Lt);
				result.push(Self::Not);
			}
			_ => {
				let val = Self::from_binary_op(op);

				result.push(val);
			},
		}

		result
	}
}
