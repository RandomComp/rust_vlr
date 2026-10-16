use std::fmt;
use std::string::FromUtf8Error;
// use std::thread::sleep;
// use std::time::Duration;

use crate::assembler::{Bytecode, VMRegs};

#[derive(Debug, Clone, thiserror::Error)]
pub enum RawVMError {
	#[error("{0}")]
	InvalidUTF8(#[from] FromUtf8Error),
	#[error("{0}")]
	Fmt(#[from] std::fmt::Error),
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

type VMWord = u32;

#[derive(Debug)]
pub struct Memory {
	mem: Vec<VMWord>,
	pub regs: VMRegs,

	heap_upper: VMWord,
	/// Indexing from heap_upper
	heap_pos: VMWord,
}

impl Memory {
	pub fn new() -> Self {
		let stack_size: VMWord = 128;
		let heap_size: VMWord = 128;

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

	pub fn push(&mut self, value: VMWord) -> Result<(), RawVMError> {
		self.regs.sp = self.regs.sp.checked_sub(1).ok_or(RawVMError::StackEmptyButExpectedValue)?;

		let result = self.mem.get_mut(self.regs.sp as usize).ok_or(RawVMError::StackOverflow)?;

		*result = value;

		Ok(())
	}

	pub fn pop(&mut self) -> Result<VMWord, RawVMError> {
		if self.regs.sp > self.regs.bp {
			return Err(RawVMError::StackEmptyButExpectedValue)
		}

		let result = self.mem.get(self.regs.sp as usize).ok_or(RawVMError::StackOverflow)?;

		self.regs.sp += 1;

		Ok(*result)
	}

	fn get_mem(&self, index: usize) -> Result<VMWord, RawVMError> {
		self.mem.get(index)
			.ok_or(RawVMError::StackOverflow)
			.copied()
	}

	fn get_mem_mut(&mut self, index: usize) -> Result<&mut VMWord, RawVMError> {
		self.mem.get_mut(index)
			.ok_or(RawVMError::StackOverflow)
	}

	fn stack_get(&self, index: usize) -> Result<&VMWord, RawVMError> {
		self.mem.get(self.regs.bp as usize + index)
			.ok_or(RawVMError::StackOverflow)
	}

	fn stack_get_mut(&mut self, index: usize) -> Result<&mut VMWord, RawVMError> {
		self.mem.get_mut(self.regs.bp as usize + index)
			.ok_or(RawVMError::StackOverflow)
	}

	fn stack_iter(&self) -> impl Iterator<Item = &VMWord> {
		self.mem.iter()
			.skip(self.regs.sp as usize)
			.take((self.regs.bp - self.regs.sp) as usize)
			.rev()
	}
}

pub struct VM {
	pub bytecode: Option<Vec<Bytecode>>,
	mem: Memory,
}

impl VM {
	// pub fn reset(&mut self) {
	//
	// }

	pub fn new(bytecode: Option<Vec<Bytecode>>) -> Self {
		Self {
			bytecode,
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

	fn exec_inst(&mut self) -> Result<bool, RawVMError> {
		let Some(bytecode) = &self.bytecode else {
			return Ok(false)
		};

		let Some(inst) = bytecode.get(self.mem.regs.pc as usize) else {
			return Ok(false)
		};

		println!("{inst}");

		if let Bytecode::Halt = inst {
			return Ok(false)
		}

		inst.exec_inst(&mut self.mem)?;

		Ok(true)
	}

	pub fn exec(&mut self) -> Result<VMWord, VMError> {
		self.mem.regs.pc = 0;

		self.dump_regs();

		loop {
			let is_not_hlt = match self.exec_inst() {
				Ok(value) => value,
				Err(e) => {
					let Some(bytecode) = &self.bytecode else {
						return Err(
							VMError {
								val: e,
								inst: None
							}
						)
					};

					let Some(inst) = bytecode.get(self.mem.regs.pc as usize) else {
						return Err(
							VMError {
								val: e,
								inst: None
							}
						)
					};

					return Err(
						VMError {
							val: e,
							inst: Some(inst.clone())
						}
					)
				}
			};

			if !is_not_hlt {
				break
			}

			self.dump_stack();
			self.dump_regs();

			// sleep(Duration::from_millis(100));
		}

		Ok(
			self.mem.get_mem(
				self.mem.regs.acc as usize
			).unwrap()
		)
	}
}
