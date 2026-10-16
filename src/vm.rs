use std::{fmt};
use std::string::FromUtf8Error;
// use std::thread::sleep;
// use std::time::Duration;

use crate::assembler::{Bytecode, VMRegs};
use crate::compiler::parser::BinaryOp;
use crate::assembler::native_func::{NativeFunctionArgsNumber, NativeFunctionID};
use crate::uni_type::{UniResult, UniResultError};
use crate::wrapper::Spanned;

#[derive(Debug, Clone, thiserror::Error)]
pub enum RawVMError {
	#[error("{0}")]
	UniResult(#[from] UniResultError),
	#[error("{0}")]
	InvalidUTF8(#[from] FromUtf8Error),
	#[error("{0}")]
	Fmt(#[from] std::fmt::Error),
	#[error("Too few arguments for native function '{id}' call, expected {expected}, found {found}")]
	ArgumentsNativeFunctionCall {id: NativeFunctionID, found: u32, expected: u32 },
	#[error("Stack empty, but expected value")]
	StackEmptyButExpectedValue,

	#[error("Stack overflowed")]
	StackOverflow,
	#[error("Invalid memory address")]
	InvalidMemoryAddress,

	#[error("Invalid stack position {0}")]
	InvalidStackPosition(usize),
	#[error("exit code: {0}")]
	Exit(UniResult),
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

#[derive(Debug)]
pub struct Memory {
	mem: Vec<UniResult>,
	pub regs: VMRegs,

	heap_upper: u32,
	/// Indexing from heap_upper
	heap_pos: u32,
}

impl Memory {
	pub fn new() -> Self {
		let stack_size: u32 = 128;
		let heap_size: u32 = 128;

		let mut regs = VMRegs::default();

		regs.bp = stack_size;
		regs.sp = regs.bp;

		let mem = vec![UniResult::default(); (stack_size + heap_size) as usize];

		Self {
			mem,
			heap_upper: stack_size,
			heap_pos: stack_size,
			regs
		}
	}

	fn push(&mut self, value: UniResult) -> Result<(), RawVMError> {
		self.regs.sp = self.regs.sp.checked_sub(1).ok_or(RawVMError::StackEmptyButExpectedValue)?;

		let result = self.mem.get_mut(self.regs.sp as usize).ok_or(RawVMError::StackOverflow)?;

		*result = value;

		Ok(())
	}

	fn pop(&mut self) -> Result<UniResult, RawVMError> {
		if self.regs.sp > self.regs.bp {
			return Err(RawVMError::StackEmptyButExpectedValue)
		}

		let result = self.mem.get(self.regs.sp as usize).ok_or(RawVMError::StackOverflow)?;

		self.regs.sp += 1;

		Ok(result.clone())
	}

	fn get_mem(&self, index: usize) -> Result<&UniResult, RawVMError> {
		self.mem.get(index)
			.ok_or(RawVMError::StackOverflow)
	}

	fn get_mem_mut(&mut self, index: usize) -> Result<&mut UniResult, RawVMError> {
		self.mem.get_mut(index)
			.ok_or(RawVMError::StackOverflow)
	}

	fn stack_get(&self, index: usize) -> Result<&UniResult, RawVMError> {
		self.mem.get(self.regs.bp as usize + index)
			.ok_or(RawVMError::StackOverflow)
	}

	fn stack_get_mut(&mut self, index: usize) -> Result<&mut UniResult, RawVMError> {
		self.mem.get_mut(self.regs.bp as usize + index)
			.ok_or(RawVMError::StackOverflow)
	}

	fn stack_iter(&self) -> impl Iterator<Item = &UniResult> {
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

	fn print(args: &[UniResult]) -> Result<UniResult, RawVMError> {
		let mut buf = String::new();

		for arg in args {
			arg.format_to_str(&mut buf)?;

			buf.push(' ');
		}

		println!("{buf}");

		Ok(UniResult::None)
	}
	fn list(args: Vec<UniResult>) -> UniResult {
		UniResult::List(
			args.into_iter().map(|v| Spanned::from_with_span(v, 0)).collect()
		)
	}
	fn range(args: &[UniResult]) -> Result<UniResult, RawVMError> {
		if args.len() <= 2 {
			unreachable!("invalid native function range call");
		}

		let start = &args[0];
		let end = &args[1];

		match (start, end) {
			(&UniResult::Float(start), &UniResult::Float(end)) =>
				Ok(UniResult::Range { start, end, step: 1.0 }),
			(&UniResult::Range { start, end, step: _ }, &UniResult::Float(step)) =>
				Ok(UniResult::Range { start, end, step }),
			(left, right) => Err(
				RawVMError::UniResult(
					UniResultError::IncompatibleOperationType {
						left: left.result_type(),
						op: BinaryOp::Range,
						right: right.result_type()
					}
				)
			),
		}
	}
	fn exit(args: &[UniResult]) -> Result<UniResult, RawVMError> {
		if args.len() != 1 {
			unreachable!("invalid native function exit call");
		}

		Err(RawVMError::Exit(args[0].clone()))
	}

	fn call_native(&mut self, id: NativeFunctionID) -> Result<UniResult, RawVMError> {
		let mut args = Vec::new();

		let expected = id.args();

		let mut i = 0;

		match expected {
			NativeFunctionArgsNumber::Fixed(fixed) => {
				while i < fixed  {
					let value = self.mem.pop().map_err(|_|
						RawVMError::ArgumentsNativeFunctionCall {
							id,
							found: i,
							expected: fixed
						}
					)?;

					args.push(value);

					i += 1;
				}
			}
			NativeFunctionArgsNumber::Dynamic => {
				if let Ok(fixed) = self.mem.pop().map(|v| i32::try_from(&v)) {
					let fixed = fixed?.cast_unsigned();

					while i < fixed  {
						let Ok(value) = self.mem.pop() else {
							return Err(
								RawVMError::ArgumentsNativeFunctionCall {
									id,
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

		match id {
			NativeFunctionID::Print =>
				Ok(Self::print(&args)?),
			NativeFunctionID::List =>
				Ok(Self::list(args)),
			NativeFunctionID::Range =>
				Self::range(&args),
			NativeFunctionID::Exit =>
				Self::exit(&args),
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

	pub fn exec(&mut self) -> Result<&UniResult, VMError> {
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
