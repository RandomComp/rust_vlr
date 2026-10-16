use std::{fmt, string::FromUtf8Error};

use crate::{native_func::{NativeFunctionID, NativeFunctionIDError}, compiler::parser::{BinaryOp, UnaryOp}};

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

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum BytecodeRegister {
	/// Subprogram result register
	Accum,
	Base,
	BasePointer,
	StackPointer,
}

impl BytecodeRegister {
	fn to_le_bytes(self) -> [u8; 1] {
		(self as u8).to_le_bytes()
	}
}

impl TryFrom<u8> for BytecodeRegister {
	type Error = BytecodeError;

	fn try_from(value: u8) -> Result<Self, Self::Error> {
		match value {
			0 => Ok(Self::Accum),
			1 => Ok(Self::Base),
			2 => Ok(Self::BasePointer),
			3 => Ok(Self::StackPointer),
			_ => Err(BytecodeError::InvalidRegisterByte(value))
		}
	}
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

fn consume_const_bytes_and_get<T, const LEN: usize>(arr: &mut T) -> Result<[u8; LEN], BytecodeError> where T: Iterator<Item=u8> {
	let mut result = [0u8; LEN];

	for val in &mut result {
		*val = arr.next().ok_or(BytecodeError::UnexpectedEof { pos: 0 })?;
	}

	Ok(result)
}

macro_rules! gen_insts_impl {
	(
		$(
			$( ( bin $binary_op:ident ) )? $( ( un $unary_op:ident ) )?
			$inst:ident $( ( $($arg:ty),* $(,)? ) )? as $main_opcode:literal $( | $( $opcode:literal )|* )?
			=>
			{
				asm 	( $result:ident $(, $($asm_arg:ident),* $(,)? )? ) => $inst_asm:block,
				disasm ( $bytes:ident ) => $inst_disasm:block,
				display ( $formatter:ident $(, $($display_arg:ident),* $(,)? )? ) => $inst_display:block
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
			fn asm_inst(&self) -> Result<Vec<u8>, BytecodeError> {
				let mut result = Vec::new();

				match self {
					$(
						Self::$inst $( ( $( $asm_arg, )* ) )? =>
							result.push($main_opcode),
					)*
				}

				match self {
					$(
						Self::$inst $( ( $( $asm_arg, )* ) )? => {
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

			pub fn to_unary_op(&self) -> Option<UnaryOp> {
				match self {
					$(
						$(
							Self::$inst => Some(UnaryOp::$unary_op),
						)?
					)*
					_ => None,
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

gen_insts_impl!(
	Halt as 0x00 | 0xFF => {
		asm(result) => {
		},
		disasm(bytes) => {
			Self::Halt
		},
		display(f) => {
			write!(f, "hlt")
		}
	},
	PushNumber(u32) as 0x01 => {
		asm(result, val) => {
			result.extend(val.to_le_bytes());
		},
		disasm(bytes) => {
			let value = u32::from_le_bytes(consume_const_bytes_and_get::<T, 4>(&mut bytes)?);

			Self::PushNumber(value)
		},
		display(f, val) => {
			write!(f, "push {val}")
		}
	},
	PushFloat(f32) as 0x02 => {
		asm(result, val) => {
			result.extend(val.to_le_bytes());
		},
		disasm(bytes) => {
			let value = f32::from_le_bytes(consume_const_bytes_and_get::<T, 4>(&mut bytes)?);

			Self::PushFloat(value)
		},
		display(f, val) => {
			write!(f, "push {val}")
		}
	},
	PushString(String) as 0x03 => {
		asm(result, text) => {
			let Ok(u32_len) = u32::try_from(text.len()) else {
				return Err(BytecodeError::StringTooLong)
			};

			result.extend(u32_len.to_le_bytes());
			result.extend(text.bytes());
		},
		disasm(bytes) => {
			let u32_len = u32::from_le_bytes(consume_const_bytes_and_get::<T, 4>(&mut bytes)?);

			let utf8 = bytes.take(u32_len as usize).collect();

			let text = String::from_utf8(utf8)?;

			Self::PushString(text)
		},
		display(f, val) => {
			write!(f, "push \"{val}\"")
		}
	},

	PushReg(BytecodeRegister) as 0x04 => {
		asm(result, reg) => {
			result.extend(reg.to_le_bytes());
		},
		disasm(bytes) => {
			let reg = BytecodeRegister::try_from(
				bytes.nth(0).ok_or(BytecodeError::UnexpectedEof { pos: 0 })?
			)?;

			Self::PushReg(reg)
		},
		display(f, reg) => {
			write!(f, "push {reg}")
		}
	},
	PopReg(BytecodeRegister) as 0x05 => {
		asm(result, reg) => {
			result.extend(reg.to_le_bytes());
		},
		disasm(bytes) => {
			let reg = BytecodeRegister::try_from(
				bytes.nth(0).ok_or(BytecodeError::UnexpectedEof { pos: 0 })?
			)?;

			Self::PopReg(reg)
		},
		display(f, reg) => {
			write!(f, "pop {reg}")
		}
	},

	Look(u32) as 0x06 => {
		asm(result, stack) => {
			result.extend(stack.to_le_bytes());
		},
		disasm(bytes) => {
			let value = u32::from_le_bytes(consume_const_bytes_and_get::<T, 4>(&mut bytes)?);

			Self::Look(value)
		},
		display(f, stack) => {
			write!(f, "look {stack}")
		}
	},
	Load(u32) as 0x07 => {
		asm(result, stack) => {
			result.extend(stack.to_le_bytes());
		},
		disasm(bytes) => {
			let value = u32::from_le_bytes(consume_const_bytes_and_get::<T, 4>(&mut bytes)?);

			Self::Load(value)
		},
		display(f, stack) => {
			write!(f, "load {stack}")
		}
	},
	Remove as 0x08 => {
		asm(_result) => {
		},
		disasm(_bytes) => {
			Self::Remove
		},
		display(f) => {
			write!(f, "remove")
		}
	},

	MovU32ToReg(BytecodeRegister, u32) as 0x0A => {
		asm(result, reg, val) => {
			result.extend(reg.to_le_bytes());
			result.extend(val.to_le_bytes());
		},
		disasm(bytes) => {
			let reg = BytecodeRegister::try_from(
				bytes.nth(0).ok_or(BytecodeError::UnexpectedEof { pos: 0 })?
			)?;

			let val = u32::from_le_bytes(consume_const_bytes_and_get::<T, 4>(&mut bytes)?);

			Self::MovU32ToReg(reg, val)
		},
		display(f, reg, val) => {
			write!(f, "mov {reg}, {val}")
		}
	},
	MovRegToReg(BytecodeRegister, BytecodeRegister) as 0x0B => {
		asm(result, reg1, reg2) => {
			result.extend(reg1.to_le_bytes());
			result.extend(reg2.to_le_bytes());
		},
		disasm(bytes) => {
			let reg1 = BytecodeRegister::try_from(
				bytes.nth(0).ok_or(BytecodeError::UnexpectedEof { pos: 0 })?
			)?;

			let reg2 = BytecodeRegister::try_from(
				bytes.nth(0).ok_or(BytecodeError::UnexpectedEof { pos: 0 })?
			)?;

			Self::MovRegToReg(reg1, reg2)
		},
		display(f, reg1, reg2) => {
			write!(f, "mov {reg1}, {reg2}")
		}
	},

	Jif(u32) as 0x20 => {
		asm(result, pos) => {
			result.extend(pos.to_le_bytes());
		},
		disasm(bytes) => {
			let pos = u32::from_le_bytes(consume_const_bytes_and_get::<T, 4>(&mut bytes)?);

			Self::Jif(pos)
		},
		display(f, pos) => {
			write!(f, "jif {pos}")
		}
	},
	Jmp(u32) as 0x28 => {
		asm(result, pos) => {
			result.extend(pos.to_le_bytes());
		},
		disasm(bytes) => {
			let pos = u32::from_le_bytes(consume_const_bytes_and_get::<T, 4>(&mut bytes)?);

			Self::Jmp(pos)
		},
		display(f, pos) => {
			write!(f, "jmp {pos}")
		}
	},
	Call(u32) as 0x2A => {
		asm(result, pos) => {
			result.extend(pos.to_le_bytes());
		},
		disasm(bytes) => {
			let pos = u32::from_le_bytes(consume_const_bytes_and_get::<T, 4>(&mut bytes)?);

			Self::Call(pos)
		},
		display(f, pos) => {
			write!(f, "call {pos}")
		}
	},
	Ret as 0x2C => {
		asm(_result) => {
		},
		disasm(_bytes) => {
			Self::Ret
		},
		display(f) => {
			write!(f, "ret")
		}
	},
	NativeCall(NativeFunctionID) as 0x80 => {
		asm(result, id) => {
			result.extend(id.to_le_bytes());
		},
		disasm(bytes) => {
			let id = NativeFunctionID::try_from(
				bytes.nth(0).ok_or(BytecodeError::UnexpectedEof { pos: 0 })?
			)?;

			Self::NativeCall(id)
		},
		display(f, id) => {
			write!(f, "native_call {id}")
		}
	},
	(bin Equals) Eq as 0x50 => {
		asm(result) => {
		},
		disasm(bytes) => {
			Self::Eq
		},
		display(f) => {
			write!(f, "eq")
		}
	},
	(bin Less) Lt as 0x52 => {
		asm(result) => {
		},
		disasm(_bytes) => {
			Self::Lt
		},
		display(f) => {
			write!(f, "lt")
		}
	},
	(bin Plus) Add as 0xA0 => {
		asm(_result) => {
		},
		disasm(_bytes) => {
			Self::Add
		},
		display(f) => {
			write!(f, "add")
		}
	},
	(bin Minus) Sub as 0xA3 => {
		asm(_result) => {
		},
		disasm(_bytes) => {
			Self::Sub
		},
		display(f) => {
			write!(f, "sub")
		}
	},
	(bin Multiply) Mul as 0xB0 => {
		asm(_result) => {
		},
		disasm(_bytes) => {
			Self::Mul
		},
		display(f) => {
			write!(f, "mul")
		}
	},
	(bin Divide) Div as 0xB3 => {
		asm(_result) => {
		},
		disasm(_bytes) => {
			Self::Div
		},
		display(f) => {
			write!(f, "div")
		}
	},
	(bin Pow) Pow as 0xC0 => {
		asm(_result) => {
		},
		disasm(_bytes) => {
			Self::Pow
		},
		display(f) => {
			write!(f, "pow")
		}
	},
	(bin LogicalAnd) LogicalAnd as 0xD3 => {
		asm(_result) => {
		},
		disasm(_bytes) => {
			Self::LogicalAnd
		},
		display(f) => {
			write!(f, "land")
		}
	},
	(bin LogicalOr) LogicalOr as 0xE0 => {
		asm(_result) => {
		},
		disasm(_bytes) => {
			Self::LogicalOr
		},
		display(f) => {
			write!(f, "lor")
		}
	},
	(un Minus) Neg as 0xC3 => {
		asm(_result) => {
		},
		disasm(_bytes) => {
			Self::Neg
		},
		display(f) => {
			write!(f, "neg")
		}
	},
	(un Not) Not as 0xD0 => {
		asm(_result) => {
		},
		disasm(_bytes) => {
			Self::Not
		},
		display(f) => {
			write!(f, "not")
		}
	},
);
