use std::string::FromUtf8Error;

use crate::compiler::native_func::{NativeFunctionIDError};

#[derive(thiserror::Error, Clone, Debug)]
pub enum BytecodeError {
	#[error("Byte {byte:02X} at {pos:02} unexpected")]
	UnexpectedByte {
		pos: usize, byte: u8
	},
	#[error("Unexpected EOF at {pos}")]
	UnexpectedEof {
		pos: usize,
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

#[derive(Copy, Clone)]
pub struct IndexedIterator<T> where T: Iterator {
	it: T,
	pub index: usize,
}

impl<T> Iterator for IndexedIterator<T> where T: Iterator {
	type Item = T::Item;

	fn next(&mut self) -> Option<Self::Item> {
		let result = self.it.next()?;

		self.index += 1;

		Some(result)
	}
}

impl<T> From<T> for IndexedIterator<T> where T: Iterator {
	fn from(it: T) -> Self {
		Self { it, index: 0 }
	}
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
		#[repr(u8)]
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
macro_rules! gen_from_bin_ops {
	( @match_body $matching_op:ident $inst:path, bin $op:path ) => {
		$inst
	};
	( @match_body $matching_op:ident $inst:path, un $op:path ) => {
		unreachable!()
	};

	( @guard_pattern bin ) => {
		true
	};
	( @guard_pattern un ) => {
		false
	};

	( @pattern bin $op:path ) => {
		$op
	};
	( @pattern un $op:path ) => {
		_
	};

	( $matching_op:ident, $( $inst:path => $attr_type:ident $( $op:ident )? ),+ $(,)? ) => {
		match $matching_op {
			$(
				$(
					gen_from_bin_ops!( @pattern $attr_type BinaryOp::$op ) if gen_from_bin_ops!( @guard_pattern $attr_type ) =>
						gen_from_bin_ops!( @match_body $matching_op $inst, $attr_type BinaryOp::$op ),
				)?
			)+
			_ =>
				todo!("'{}' not handled in from_binary_op or unreachable", $matching_op)
		}
	};
}

#[macro_export]
macro_rules! gen_from_un_ops {
	( @match_body $matching_op:ident $inst:path, bin $op:path ) => {
		unreachable!()
	};
	( @match_body $matching_op:ident $inst:path, un $op:path ) => {
		$inst
	};

	( @guard_pattern bin ) => {
		false
	};
	( @guard_pattern un ) => {
		true
	};

	( bin $op:path ) => {
		_
	};
	( un $op:path ) => {
		$op
	};
	( $matching_op:ident, $( $inst:path => $attr_type:ident $( $op:ident )? ),+ $(,)? ) => {
		match $matching_op {
			$(
				$(
					gen_from_un_ops!( $attr_type UnaryOp::$op ) if gen_from_un_ops!( @guard_pattern $attr_type ) =>
						gen_from_un_ops!( @match_body $matching_op $inst, $attr_type UnaryOp::$op ),
				)?
			)+
			_ => {
				todo!("'{}' not handled in from_unary_op or unreachable", $matching_op)
			}
		}
	};
}

#[macro_export]
macro_rules! gen_insts_impl {
	( @text_if_pseudo pseudo ) => {
		concat!(
			"# Important\n\r",
			"This is a pseudo instruction, this instrunction doesn't have a disasm or execution function implementation"
		)
	};
	( @text_if_pseudo bin $op:ident ) => {
		concat!(
			"# Note\n\r",
			"This is a binary '", stringify!($op), "' mathemetical instruction"
		)
	};
	( @text_if_pseudo un $op:ident ) => {
		concat!(
			"# Note\n\r",
			"This is a unary '", stringify!($op), "' mathemetical instruction"
		)
	};
	( @text_if_pseudo ) => {
		""
	};

	(
		$(
			$( #[doc=$inst_doc:literal] )+ $( ( $( $attr:ident )+ ) )? $inst:ident
			$( ( $($arg:ty),* $(,)? ) )? $( as $main_opcode:literal $( | $opcode:literal )* )?
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
		use $crate::assembler::defs::IndexedIterator;

		#[derive(PartialEq, Clone, Debug)]
		pub enum Bytecode {
			$(
				$(
					#[doc=$inst_doc]
				)*
				#[doc=concat!(
					$(
						"# Where \n",
						$(
							"* ", stringify!($asm_arg), ": [`", stringify!($arg), "`]\n",
						)*
					)?
				)]
				$(
					#[doc=concat!(
						"# Opcodes \n",
						"* main opcode: `", stringify!($main_opcode), "`\n",
						$( "* other possible opcode: `", stringify!($opcode), "`\n", )*
					)]
				)?
				$(
					#[doc=
						gen_insts_impl!( @text_if_pseudo $( $attr )+ )
					]
				)?
				$inst $(
					( $( $arg, )* )
				)?,
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
			fn asm_inst(self) -> Result<Vec<u8>, BytecodeError> {
				let mut result = Vec::new();

				match self {
					$(
						Self::$inst $( ( $( $asm_arg, )* ) )? => {
							$(
								result.push($main_opcode);
							)?

							#[allow(unused_mut)]
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

			pub fn disasm_inst<T>(bytes: T) -> Result<Option<(Self, usize)>, BytecodeError>
				where T: Iterator<Item = u8> {
				let mut bytes = IndexedIterator::from(bytes);

				let Some(opcode) = bytes.next() else {
					return Ok(None)
				};

				match opcode {
					$(
						$(
							$main_opcode $( | $opcode )* => {
								#[allow(unused_mut)]
								let mut $bytes = bytes;

								let result = $inst_disasm;

								Ok(Some((result, $bytes.index)))
							},
						)?
					)*
					x => Err(BytecodeError::UnexpectedByte { pos: 0, byte: x }), // TODO: fix: pos: 0 in bytecode disasembler
				}
			}

			#[allow(dead_code)]
			pub fn disasm<T>(bytes: T) -> Result<Vec<Self>, BytecodeError> where T: Iterator<Item = u8> + fmt::Debug + Copy {
				let mut result = Vec::new();

				let bytes = IndexedIterator::from(bytes);

				while let Some(val) = Self::disasm_inst(bytes)? {
					result.push(val.0);
				}

				Ok(result)
			}

			pub fn exec(&self, mem: &mut Memory) -> Result<(), RawVMError> {
				match self {
					$(
						#[allow(unused_variables)]
						&Self::$inst $( ( $( $exec_arg, )* ) )? => {
							#[allow(unused_mut)]
							let mut $mem = mem;

							$inst_exec;

							#[allow(unreachable_code)]
							Ok(())

							// result
						},
					)*
				}
			}

			pub fn len(&self) -> usize {
				match self {
					$(
						Self::$inst $( ( $( $exec_arg, )* ) )? => {
							1 $( $( + std::mem::size_of::<$arg>() )* )?
						}
					)*
				}
			}

			pub fn from_unary_op(op: &UnaryOp) -> Bytecode {
				gen_from_un_ops!(op,
					$(
						$(
							Bytecode::$inst => $( $attr )+,
						)?
					)*
				)
			}

			pub fn from_binary_op(op: &BinaryOp) -> Bytecode {
				gen_from_bin_ops!(op,
					$(
						$(
							Bytecode::$inst => $( $attr )+,
						)?
					)*
				)
			}
		}
	};
}
