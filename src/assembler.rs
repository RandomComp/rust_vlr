use crate::{assembler::{defs::BytecodeError, native_func::NativeFunctionID}, gen_insts_impl, gen_regs_code};

pub mod native_func;
pub mod defs;
pub mod optimizer;

use std::fmt;

/*
Bytecode::Halt =>
						return Ok(false),
					&Bytecode::PushNumber(val) => {
						self.stack_push(val.into())?;
					},
					&Bytecode::PushFloat(val) => {
						self.stack_push(val.into())?;
					},
					Bytecode::PushString(val) => {
						self.stack_push(val.clone().into())?;
					},

					&Bytecode::MovRegToReg(reg1, reg2) => {
						let reg2 = self.regs[reg2];

						self.regs[reg1] = reg2;
					},
					&Bytecode::MovU32ToReg(reg, value) => {
						self.regs[reg] = value;
					},

					&Bytecode::PushReg(reg) => {
						let value = self.regs[reg];

						self.stack_push(value.into())?;
					},
					&Bytecode::PopReg(reg) => {
						let value = u32::try_from(&self.stack_pop()?)?;

						self.regs[reg] = value;
					},

					&Bytecode::Look(stack_pos) => {
						let usize_stack_pos = stack_pos as usize;

						let val = self.stack_get(usize_stack_pos)?;

						self.stack_push(val.clone())?;
					},
					&Bytecode::Load(stack_pos) => {
						let usize_stack_pos = stack_pos as usize;

						let value = self.stack_pop()?;

						let val = self.stack_get_mut(usize_stack_pos)?;

						*val = value;
					},
					Bytecode::Remove => {
						self.stack_pop()?;
					},
					&Bytecode::Jif(dest_pc) => {
						if !bool::try_from(self.stack_pop()?)? {
							pc = dest_pc;
						}
					},
					&Bytecode::Jmp(dest_pc) => {
						pc = dest_pc;
					},
					&Bytecode::Call(dest_pc) => {
						self.stack_push((self.regs.pc + 1).into())?;

						pc = dest_pc;
					},
					&Bytecode::Ret => {
						pc = u32::try_from(&self.stack_pop()?)?;
					},
					&Bytecode::NativeCall(id) => {
						let result = self.call_native(id)?;

						let mem = self.get_mem_mut(self.heap_pos as usize)?;

						*mem = result;

						self.regs.acc = self.heap_pos;

						self.heap_pos += 1;
					},
					Bytecode::Eq |
					Bytecode::Lt |
				 	Bytecode::Add |
					Bytecode::Sub |
					Bytecode::Mul |
					Bytecode::Pow |
					Bytecode::Div |
					Bytecode::LogicalAnd |
					Bytecode::LogicalOr => {
						let op = &inst.to_binary_op().unwrap_or_else(|| unreachable!("invalid binary operation {inst} in match"));

						let right = self.get_mem(self.regs.base as usize)?.clone();
						let left = self.get_mem_mut(self.regs.acc as usize)?;

						let result = UniResult::calc_binary_by_op(
							&Spanned::from_with_span(left.clone(), 0),
							&Spanned::from_with_span(right.clone(), 0),
							op
						).map(|v| v.val).map_err(|e| e.val)?;

						*left = result;
					},
					Bytecode::Neg |
					Bytecode::Not => {
						let op = &inst.to_unary_op().unwrap_or_else(|| unreachable!("invalid unary operation {inst} in match"));

						let value = self.stack_pop()?;

						let result = UniResult::calc_unary_by_op(
							&Spanned::from_with_span(value, 0),
							op
						).map(|v| v.val).map_err(|e| e.val)?;

						self.stack_push(result)?;
					},
 */


fn consume_const_bytes_and_get<T, const LEN: usize>(arr: &mut T) -> Result<[u8; LEN], BytecodeError> where T: Iterator<Item=u8> {
	let mut result = [0u8; LEN];

	for val in &mut result {
		*val = arr.next().ok_or(BytecodeError::UnexpectedEof { pos: 0 })?;
	}

	Ok(result)
}

gen_regs_code!(
	0 | Accum 			=> acc,
	1 | Base 			=> base,
	2 | BasePointer 	=> bp,
	3 | StackPointer 	=> sp,
	4 | ProgramCounter 	=> pc,
);

gen_insts_impl!(
	Halt as 0x00 | 0xFF => {
		asm(result) => {
		},
		disasm(bytes) => {
			Self::Halt
		},
		exec(mem) => {
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
		exec(mem, val) => {
			mem.push(val)?;
		},
		display(f, val) => {
			write!(f, "push {val}")
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
		exec(mem, reg) => {
			mem.push(mem.regs[reg])?;
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
		exec(mem, reg) => {
			mem.regs[reg] = mem.pop()?;
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
		exec(mem, stack) => {

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
		exec(mem,stack) => {

		},
		display(f, stack) => {
			write!(f, "load {stack}")
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
		exec(mem, reg, val) => {
			mem.regs[reg] = val;
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
		exec(mem, reg1, reg2) => {
			mem.regs[reg1] = mem.regs[reg2];
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
		exec(mem, pos) => {
			todo!()
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
		exec(mem, pos) => {
			mem.regs.pc = pos;
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
		exec(mem, pos) => {
			mem.push(mem.regs.pc)?;

			mem.regs.pc = pos;
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
		exec(mem) => {
			mem.regs.pc = mem.pop()?;
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
		exec(mem, id) => {

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
		exec(mem) => {
			let right = mem.regs.base;

			mem.regs.acc = (mem.regs.acc == right).into();
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
		exec(mem) => {
			let right = mem.regs.base;

			mem.regs.acc = (mem.regs.acc < right).into();
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
		exec(mem) => {
			let right = mem.regs.base;

			mem.regs.acc += right;
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
		exec(mem) => {
			let right = mem.regs.base;

			mem.regs.acc -= right;
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
		exec(mem) => {
			let right = mem.regs.base;

			mem.regs.acc *= right;
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
		exec(mem) => {
			let right = mem.regs.base;

			mem.regs.acc /= right;
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
		exec(mem) => {
			let right = mem.regs.base;

			mem.regs.acc = mem.regs.acc.pow(right);
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
		exec(mem) => {
			let right = mem.regs.base;

			mem.regs.acc = (mem.regs.acc != 0 && right != 0).into();
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
		exec(mem) => {
			let right = mem.regs.base;

			mem.regs.acc = (mem.regs.acc != 0 && right != 0).into();
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
		exec(mem) => {
			mem.regs.acc = (mem.regs.acc | (1 << 31)) + 1;
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
		exec(mem) => {
			mem.regs.acc = (mem.regs.acc == 0).into();
		},
		display(f) => {
			write!(f, "not")
		}
	},
);
