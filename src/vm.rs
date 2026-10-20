use std::fmt;
use std::slice::SliceIndex;
use std::string::FromUtf8Error;
// use std::thread::sleep;
// use std::time::Duration;

use crate::assembler::defs::{BytecodeError, IndexedIterator};
use crate::assembler::{Bytecode, VMRegs};
use crate::compiler::{native_func::NativeFunctionIDError};

#[derive(Debug, Clone, thiserror::Error)]
pub enum RawVMError {
	#[error("{0}")]
	InvalidUTF8(#[from] FromUtf8Error),
	#[error("{0}")]
	Fmt(#[from] std::fmt::Error),
	#[error("{0}")]
	NativeFunction(#[from] NativeFunctionIDError),
	#[error("{0}")]
	Bytecode(#[from] BytecodeError),
	#[error("Stack empty, but expected value")]
	StackEmptyButExpectedValue,

	#[error("Stack overflowed")]
	StackOverflow,
	#[error("Invalid memory address")]
	InvalidMemoryAddress,
}

#[derive(Debug, Clone)]
pub struct VMError {
	pub val: RawVMError,
	inst: Option<Bytecode>,
}

impl fmt::Display for VMError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		if let Some(inst) = &self.inst {
			write!(f, "While executing instruction \"{inst}\", an error occured: {}", self.val)
		} else {
			write!(f, "While executing instruction, an error occured: {}", self.val)
		}
	}
}

pub type VMWord = u32;

#[derive(Debug)]
pub struct Memory {
	mem: Vec<u8>,
	pub regs: VMRegs,

	heap_upper: VMWord,
	/// Indexing from `heap_upper`
	heap_pos: VMWord,
}

impl Memory {
	pub fn new() -> Self {
		let stack_size: VMWord = 128;
		let heap_size: VMWord = 1024;

		let mut regs = VMRegs::default();

		regs.bp = stack_size;
		regs.sp = regs.bp;

		let mem = vec![0; (stack_size + heap_size) as usize];

		Self {
			mem,
			heap_upper: stack_size,
			heap_pos: stack_size,
			regs
		}
	}

	pub fn place_to_heap(&mut self, values: &[u8]) -> Result<u32, RawVMError> {
		let pos = self.heap_pos;

		for &v in values {
			*(self.get_mem_mut(self.heap_pos as usize)?) = v;

			self.heap_pos += 1;
		}

		Ok(pos)
	}

	pub fn get_word(&self, index: usize) -> Result<VMWord, RawVMError> {
		let result = self.mem_iter(index)
						.take(4)
						.collect::<Vec<u8>>();

		let result = u32::from_le_bytes(
			result[0..4].try_into().map_err(|_| RawVMError::InvalidMemoryAddress)?
		);

		Ok(result)
	}

	pub fn set_word(&mut self, index: usize, val: VMWord) -> Result<(), RawVMError> {
		for (i, &byte) in val.to_le_bytes().iter().enumerate() {
			let result = self.mem.get_mut(index + i).ok_or(RawVMError::InvalidMemoryAddress)?;

			*result = byte;
		}

		Ok(())
	}

	pub fn push(&mut self, value: VMWord) -> Result<(), RawVMError> {
		self.regs.sp = self.regs.sp.checked_sub(4).ok_or(RawVMError::StackOverflow)?;

		self.set_word(self.regs.sp as usize, value)?;

		Ok(())
	}

	pub fn pop(&mut self) -> Result<VMWord, RawVMError> {
		if (self.regs.sp + 4) > self.regs.bp {
			return Err(RawVMError::StackEmptyButExpectedValue)
		}

		let result = self.get_word(self.regs.sp as usize)?;

		self.regs.sp += 4;

		Ok(result)
	}

	pub fn get_mem<I>(&self, index: I) -> Result<u8, RawVMError>
		where I: SliceIndex<[u8], Output = u8> {
		self.mem.get(index)
			.ok_or(RawVMError::InvalidMemoryAddress)
			.copied()
	}

	fn get_mem_mut<I>(&mut self, index: I) -> Result<&mut u8, RawVMError>
		where I: SliceIndex<[u8], Output = u8> {
		self.mem.get_mut(index)
			.ok_or(RawVMError::InvalidMemoryAddress)
	}

	pub fn mem_iter(&self, offset: usize) -> impl Iterator<Item = u8> {
		self.mem.iter().skip(offset).copied()
	}

	fn stack_iter(&self) -> impl Iterator<Item = VMWord> {
		self.mem.chunks(4)
			.skip((self.regs.sp as usize) / 4)
			.take(((self.regs.bp - self.regs.sp) as usize) / 4)
			.map(|v| VMWord::from_le_bytes(
				v[0..4].try_into().unwrap()
			))
			.rev()
	}
}

pub struct VM {
	pub mem: Memory,
}

impl VM {
	// pub fn reset(&mut self) {
	//
	// }

	pub fn new() -> Self {
		Self {
			mem: Memory::new(),
		}
	}

	fn dump_regs(&self) {
		println!("{}", self.mem.regs);
	}

	fn dump_stack(&self) {
		print!("stack = [");

		for (i, value) in self.mem.stack_iter().enumerate() {
			if i > 0 {
				print!(", ");
			}

			print!("{value}");
		}

		println!("]");
	}

	fn exec_inst(&mut self) -> Result<bool, VMError> {
		let bytes = self.mem.mem_iter(self.mem.regs.pc as usize);

		let inst_opt = Bytecode::disasm_inst(bytes).map_err(|e| VMError {
			val: e.into(),
			inst: None
		})?;

		let Some((inst, inst_size)) = inst_opt else {
			return Ok(false);
		};

		// println!("{inst}");

		if let Bytecode::Halt = inst {
			return Ok(false)
		}

		let pc = self.mem.regs.pc + (inst_size as VMWord);
		self.mem.regs.pc = pc;

		inst.exec(&mut self.mem).map_err(|e| VMError {
			val: e,
			inst: Some(inst)
		})?;

		Ok(true)
	}

	pub fn exec(&mut self) -> Result<VMWord, VMError> {
		self.dump_regs();

		println!();

		loop {
			let is_not_hlt = self.exec_inst()?;

			if !is_not_hlt {
				break
			}

			// self.dump_stack();
			// self.dump_regs();

			// println!();

			// sleep(Duration::from_millis(100));
		}

		Ok(
			self.mem.get_word(
				self.mem.regs.acc as usize
			).unwrap()
		)
	}
}
