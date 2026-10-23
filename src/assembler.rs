use crate::vm::VMSWord;
use crate::{gen_from_bin_ops, gen_from_un_ops};

use crate::{{assembler::defs::BytecodeError}, {compiler::native_func::NativeFunctionID}, gen_insts_impl, gen_regs_code};

pub mod defs;
pub mod optimizer;
pub mod linker;

use std::fmt;

fn consume_const_bytes_and_get<const LEN: usize, T>(arr: &mut T) -> Result<[u8; LEN], BytecodeError>
	where T: Iterator<Item=u8> {
	let mut result = [0u8; LEN];

	for val in &mut result {
		*val = arr.next().ok_or(BytecodeError::UnexpectedEof { pos: 0 })?;
	}

	Ok(result)
}

gen_regs_code!(
	/// Accumulator register (use it for first operand and save it to)
	0 | Accum 			=> acc,
	/// Base register (use it for second operand)
	1 | Base 			=> base,
	/// Loop counter (prefer use it for loop's)
	2 | Counter 		=> cnt,
	/// Base stack pointer, points to stack frame begin
	3 | BasePointer 	=> bp,
	/// Stack pointer, points to current stack element
	4 | StackPointer 	=> sp,
	/// Program counter, points to next instruction
	5 | ProgramCounter 	=> pc,
);

gen_insts_impl!(
	/// # Do nothing.
	/// # Example
	/// ```asm,ignore
	/// nop ; do nothings
	///
	/// nop
	/// nop
	/// nop
	/// nop
	/// ...
	/// ; repeating doesn't do anything
	/// ```
	Nop as 0x90 => {
		asm(result) => {
		},
		disasm(bytes) => {
			Self::Nop
		},
		exec(mem) => {
		},
		display(f) => {
			write!(f, "nop")
		}
	},

	/// # Halts program
	/// # Example
	/// ```asm,ignore
	/// hlt
	/// ; intentionally unreachable
	/// mov acc, 0
	/// mov base, 0
	/// div
	/// ```
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

	/// # Push reference offset to stack
	/// # Example
	/// ```asm,ignore
	/// push ref [data+0x10] ; push reference with offset 0x10 of data segment to stack
	/// ```
	(pseudo) PushRef(VMWord) => {
		display(f, val) => {
			write!(f, "push ref [data{val:+}]")
		}
	},

	/// # Push `val` number to stack
	/// # Example
	/// ```asm,ignore
	/// push 123 ; push 123 to stack
	/// ```
	PushNumber(VMWord) as 0x01 => {
		asm(result, val) => {
			result.extend(val.to_le_bytes());
		},
		disasm(bytes) => {
			let value = VMWord::from_le_bytes(consume_const_bytes_and_get::<4, _>(&mut bytes)?);

			Self::PushNumber(value)
		},
		exec(mem, val) => {
			mem.push(val)?;
		},
		display(f, val) => {
			write!(f, "push {val}")
		}
	},

	/// # Push `reg` value to stack
	/// # Example
	/// ```asm,ignore
	/// mov acc, 123
	/// push acc
	/// pop base ; in base now 123
	/// ```
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

	/// # Pop from stack to `reg`
	/// # Example
	/// ```asm,ignore
	/// push 123
	/// pop acc ; in acc now 123
	/// ```
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

	/// # Loads `[reg2 +/- offset]` to `reg1`
	/// # Example
	/// ```asm,ignore
	/// mov base, 0x16 ; heap address, 0x16 + 128 = 150 = 0x96
	/// mov acc, [base+0x80] ; move heap 0x16 value to acc
	/// ; in acc our value
	/// ```
	LoadToReg(BytecodeRegister, BytecodeRegister, VMSWord) as 0x06 => {
		asm(result, reg1, reg2, offset) => {
			result.extend(reg1.to_le_bytes());
			result.extend(reg2.to_le_bytes());
			result.extend(offset.to_le_bytes());
		},
		disasm(bytes) => {
			let reg1 = BytecodeRegister::try_from(
				bytes.nth(0).ok_or(BytecodeError::UnexpectedEof { pos: 0 })?
			)?;
			let reg2 = BytecodeRegister::try_from(
				bytes.nth(0).ok_or(BytecodeError::UnexpectedEof { pos: 0 })?
			)?;

			let offset = VMSWord::from_le_bytes(consume_const_bytes_and_get::<4, _>(&mut bytes)?);

			Self::LoadToReg(reg1, reg2, offset)
		},
		exec(mem, reg1, reg2, offset) => {
			let addr = (mem.regs[reg2].strict_sub_signed(offset)) as usize;

			let value = mem.get_word(addr)?;

			mem.regs[reg1] = value;
		},
		display(f, reg1, reg2, offset) => {
			write!(f, "mov {reg1}, [{reg2}{offset:+}]")
		}
	},

	/// # Loads `val_reg` value to `[reg +/- offset]`
	/// # Example
	/// ```asm,ignore
	/// mov acc, 123 ; value
	/// mov base, 0x16 ; heap address, 0x16 + 128 = 150 = 0x96
	/// mov [base+0x80], acc ; move acc value to heap 0x16
	/// ```
	LoadFromReg(BytecodeRegister, BytecodeRegister, VMSWord) as 0x07 => {
		asm(result, val_reg, reg, offset) => {
			result.extend(val_reg.to_le_bytes());
			result.extend(reg.to_le_bytes());
			result.extend(offset.to_le_bytes());
		},
		disasm(bytes) => {
			let val_reg = BytecodeRegister::try_from(
				bytes.nth(0).ok_or(BytecodeError::UnexpectedEof { pos: bytes.index })?
			)?;
			let reg = BytecodeRegister::try_from(
				bytes.nth(0).ok_or(BytecodeError::UnexpectedEof { pos: bytes.index })?
			)?;

			let offset = VMSWord::from_le_bytes(consume_const_bytes_and_get::<4, _>(&mut bytes)?);

			Self::LoadFromReg(val_reg, reg, offset)
		},
		exec(mem, val_reg, reg, offset) => {
			let value = mem.regs[val_reg];

			let addr = (mem.regs[reg].strict_sub_signed(offset)) as usize;

			mem.set_word(addr, value)?;
		},
		display(f, val_reg, reg, offset) => {
			write!(f, "mov [{reg}{offset:+}], {val_reg}")
		}
	},

	/// # Moves `val` constant to `reg`
	/// # Example
	/// ```asm,ignore
	/// mov acc, 123
	/// ```
	MovU32ToReg(BytecodeRegister, VMWord) as 0x0A => {
		asm(result, reg, val) => {
			result.extend(reg.to_le_bytes());
			result.extend(val.to_le_bytes());
		},
		disasm(bytes) => {
			let reg = BytecodeRegister::try_from(
				bytes.nth(0).ok_or(BytecodeError::UnexpectedEof { pos: 0 })?
			)?;

			let val = VMWord::from_le_bytes(consume_const_bytes_and_get::<4, _>(&mut bytes)?);

			Self::MovU32ToReg(reg, val)
		},
		exec(mem, reg, val) => {
			mem.regs[reg] = val;
		},
		display(f, reg, val) => {
			write!(f, "mov {reg}, {val}")
		}
	},

	/// # Move `reg2` value to `reg1`
	/// # Example
	/// ```asm,ignore
	/// mov acc, base
	/// ```
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

	/// # Conditional jump if `acc == 0` on address `pos`
	/// # Example
	/// ```asm,ignore
	/// mov cnt, 0 ; our counter
	/// mov base, 10
	///
	/// some_label: ; our label
	/// mov acc, cnt
	/// lt
	/// jif loop_exit ; our conditional jump
	/// add cnt, 1
	/// jmp some_label
	/// loop_exit:
	/// ```
	Jif(VMWord) as 0x20 => {
		asm(result, pos) => {
			result.extend(pos.to_le_bytes());
		},
		disasm(bytes) => {
			let pos = VMWord::from_le_bytes(consume_const_bytes_and_get::<4, _>(&mut bytes)?);

			Self::Jif(pos)
		},
		exec(mem, pos) => {
			todo!()
		},
		display(f, pos) => {
			write!(f, "jif {pos}")
		}
	},

	/// # Unconditional jump on label `label`
	/// # Example
	/// ```asm,ignore
	/// mov cnt, 0 ; our counter
	/// mov base, 10
	///
	/// some_label: ; our label
	/// mov acc, cnt
	/// lt
	/// jif loop_exit
	/// add cnt, 1
	/// jmp some_label ; jump on some_label
	/// loop_exit:
	/// ```
	(pseudo) JmpLabel(usize) => {
		display(f, label) => {
			write!(f, "jmp {label}")
		}
	},

	/// # Push pc to stack and jump on label `label`
	/// # Example
	/// ```asm,ignore
	/// some_subprogram: ; our subprogram
	/// push bp
	/// mov bp, sp
	/// ; ... some instructions ...
	/// pop bp
	/// ret ; returns from subprogram to where called
	/// call some_subprogram ; our calling
	/// ```
	(pseudo) CallLabel(usize) => {
		display(f, label) => {
			write!(f, "call {label}")
		}
	},

	/// # Unconditional jump on address `pos`
	/// # Example
	/// ```asm,ignore
	/// mov cnt, 0 ; our counter
	/// mov base, 10
	///
	/// some_label: ; our label
	/// mov acc, cnt
	/// lt
	/// jif loop_exit
	/// add cnt, 1
	/// jmp some_label ; jump on some_label
	/// loop_exit:
	/// ```
	/// # Safety
	/// This is not safe, because it may jump and execute every data from memory!
	/// If you wanna use this for loops or another, use pseudo Jmp with labels instead
	Jmp(VMWord) as 0x28 => {
		asm(result, pos) => {
			result.extend(pos.to_le_bytes());
		},
		disasm(bytes) => {
			let pos = VMWord::from_le_bytes(consume_const_bytes_and_get::<4, _>(&mut bytes)?);

			Self::Jmp(pos)
		},
		exec(mem, pos) => {
			mem.regs.pc = pos;
		},
		display(f, pos) => {
			write!(f, "jmp {pos}")
		}
	},

	/// # Push pc to stack and jump on address `pos`
	/// # Example
	/// ```asm,ignore
	/// some_subprogram: ; our subprogram
	/// push bp
	/// mov bp, sp
	/// ; ... some instructions ...
	/// pop bp
	/// ret ; returns from subprogram to where called
	/// call some_subprogram ; our calling
	/// ```
	/// # Safety
	/// This is not safe, because it may call like function and execute every data from memory!
	/// If you wanna use this for function call, use pseudo Call with labels instead
	Call(VMWord) as 0x2A => {
		asm(result, pos) => {
			result.extend(pos.to_le_bytes());
		},
		disasm(bytes) => {
			let pos = VMWord::from_le_bytes(consume_const_bytes_and_get::<4, _>(&mut bytes)?);

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

	/// # Pop address from stack and jumps on it:
	/// # Example
	/// ```asm,ignore
	/// some_subprogram: ; our subprogram
	/// push bp
	/// mov bp, sp
	/// ; ... some instructions ...
	/// pop bp
	/// ret ; returns from subprogram to where called
	/// call some_subprogram ; our calling
	/// ```
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

	/// # Call built-in function `id`:
	/// # Example
	/// ```asm,ignore
	/// mov acc, 1 ; our exit code
	/// native exit ; exit from program
	/// ```
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
			id.call(mem)?;
		},
		display(f, id) => {
			write!(f, "native {id}")
		}
	},

	/// # Binary equal, calculates:
	/// ```asm,ignore
	/// reg1 = reg1 == reg2
	/// ```
	/// # Example
	/// ```asm,ignore
	/// mov acc, 2
	/// mov base, 2
	/// eq
	/// ```
	(bin Equals) Eq(BytecodeRegister, BytecodeRegister) as 0x50 => {
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

			Self::Eq(reg1, reg2)
		},
		exec(mem, reg1, reg2) => {
			let right = mem.regs[reg2];

			mem.regs[reg1] = (mem.regs[reg1] == right).into();
		},
		display(f, reg1, reg2) => {
			write!(f, "eq {reg1}, {reg2}")
		}
	},

	/// # Binary less, calculates:
	/// ```asm,ignore
	/// acc = acc < base
	/// ```
	/// # Example
	/// ```asm,ignore
	/// mov acc, 2
	/// mov base, 3
	/// lt
	/// ```
	(bin Less) Lt(BytecodeRegister, BytecodeRegister) as 0x52 => {
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

			Self::Lt(reg1, reg2)
		},
		exec(mem, reg1, reg2) => {
			let right = mem.regs[reg2];

			mem.regs[reg1] = (mem.regs[reg1] < right).into();
		},
		display(f, reg1, reg2) => {
			write!(f, "lt {reg1}, {reg2}")
		}
	},

	/// # Binary addition, calculates:
	/// ```asm,ignore
	/// acc = acc + base
	/// ```
	/// # Example
	/// ```asm,ignore
	/// mov acc, 2
	/// mov base, 2
	/// add
	/// ```
	(bin Plus) Add(BytecodeRegister, BytecodeRegister) as 0xA0 => {
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

			Self::Add(reg1, reg2)
		},
		exec(mem, reg1, reg2) => {
			let right = mem.regs[reg2];

			mem.regs[reg1] += right;
		},
		display(f, reg1, reg2) => {
			write!(f, "add {reg1}, {reg2}")
		}
	},

	/// # Binary subtraction, calculates:
	/// ```asm,ignore
	/// acc = acc - base
	/// ```
	/// # Example
	/// ```asm,ignore
	/// mov acc, 123
	/// mov base, 3
	/// sub
	/// ```
	(bin Minus) Sub(BytecodeRegister, BytecodeRegister) as 0xA3 => {
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

			Self::Sub(reg1, reg2)
		},
		exec(mem, reg1, reg2) => {
			let right = mem.regs[reg2];

			mem.regs[reg1] -= right;
		},
		display(f, reg1, reg2) => {
			write!(f, "sub {reg1}, {reg2}")
		}
	},

	/// # Binary multiplication, calculates:
	/// ```asm,ignore
	/// acc = acc * base
	/// ```
	/// # Example
	/// ```asm,ignore
	/// mov acc, 2
	/// mov base, 3
	/// mul
	/// ```
	(bin Multiply) Mul(BytecodeRegister, BytecodeRegister) as 0xB0 => {
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

			Self::Mul(reg1, reg2)
		},
		exec(mem, reg1, reg2) => {
			let right = mem.regs[reg2];

			mem.regs[reg1] *= right;
		},
		display(f, reg1, reg2) => {
			write!(f, "mul {reg1}, {reg2}")
		}
	},

	/// # Binary divizion, calculates:
	/// ```asm,ignore
	/// acc = acc / base
	/// ```
	/// # Example
	/// ```asm,ignore
	/// mov acc, 2
	/// mov base, 3
	/// div
	/// ```
	(bin Divide) Div(BytecodeRegister, BytecodeRegister) as 0xB3 => {
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

			Self::Div(reg1, reg2)
		},
		exec(mem, reg1, reg2) => {
			let right = mem.regs[reg2];

			mem.regs[reg1] /= right;
		},
		display(f, reg1, reg2) => {
			write!(f, "div {reg1}, {reg2}")
		}
	},

	/// # Binary power, calculates:
	/// ```asm,ignore
	/// acc = acc ** base
	/// ```
	/// # Example
	/// ```asm,ignore
	/// mov acc, 2
	/// mov base, 3
	/// pow
	/// ```
	(bin Pow) Pow(BytecodeRegister, BytecodeRegister) as 0xC0 => {
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

			Self::Pow(reg1, reg2)
		},
		exec(mem, reg1, reg2) => {
			let right = mem.regs[reg2];

			mem.regs[reg1] = mem.regs[reg1].pow(right);
		},
		display(f, reg1, reg2) => {
			write!(f, "pow {reg1}, {reg2}")
		}
	},

	/// # Logical and, calculates:
	/// ```asm,ignore
	/// acc = acc != 0 && base != 0
	/// ```
	/// # Example
	/// ```asm,ignore
	/// mov acc, 1
	/// mov base, 1
	/// land
	/// ```
	(bin LogicalAnd) LogicalAnd(BytecodeRegister, BytecodeRegister) as 0xD3 => {
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

			Self::LogicalAnd(reg1, reg2)
		},
		exec(mem, reg1, reg2) => {
			let right = mem.regs[reg2];

			mem.regs[reg1] = (mem.regs[reg1] != 0 && right != 0).into();
		},
		display(f, reg1, reg2) => {
			write!(f, "land {reg1}, {reg2}")
		}
	},

	/// # Logical or, calculates:
	/// ```asm,ignore
	/// acc = acc != 0 || base != 0
	/// ```
	/// # Example
	/// ```asm,ignore
	/// mov acc, 1
	/// mov base, 1
	/// lor
	/// ```
	(bin LogicalOr) LogicalOr(BytecodeRegister, BytecodeRegister) as 0xE0 => {
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

			Self::LogicalOr(reg1, reg2)
		},
		exec(mem, reg1, reg2) => {
			let right = mem.regs[reg2];

			mem.regs[reg1] = (mem.regs[reg1] != 0 || right != 0).into();
		},
		display(f, reg1, reg2) => {
			write!(f, "lor {reg1}, {reg2}")
		}
	},

	/// # Unary negation, calculates:
	/// ```asm,ignore
	/// acc = -acc
	/// ```
	/// # Example
	/// ```asm,ignore
	/// mov acc, 123
	/// neg
	/// ```
	(un Minus) Neg(BytecodeRegister) as 0xC3 => {
		asm(result, reg) => {
			result.extend(reg.to_le_bytes());
		},
		disasm(bytes) => {
			let reg = BytecodeRegister::try_from(
				bytes.nth(0).ok_or(BytecodeError::UnexpectedEof { pos: 0 })?
			)?;

			Self::Neg(reg)
		},
		exec(mem, reg) => {
			mem.regs[reg] = mem.regs[reg].strict_neg();
		},
		display(f, reg) => {
			write!(f, "neg {reg}")
		}
	},

	/// # Logical not, calculates:
	/// ```asm,ignore
	/// acc = acc == 0
	/// ```
	/// # Example
	/// ```asm,ignore
	/// mov acc, 0
	/// not
	/// ```
	(un Not) Not(BytecodeRegister) as 0xD0 => {
		asm(result, reg) => {
			result.extend(reg.to_le_bytes());
		},
		disasm(bytes) => {
			let reg = BytecodeRegister::try_from(
				bytes.nth(0).ok_or(BytecodeError::UnexpectedEof { pos: 0 })?
			)?;

			Self::Not(reg)
		},
		exec(mem, reg) => {
			mem.regs[reg] = (mem.regs[reg] == 0).into();
		},
		display(f, reg) => {
			write!(f, "not {reg}")
		}
	},
);
