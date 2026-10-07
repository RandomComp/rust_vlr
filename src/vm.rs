use std::string::FromUtf8Error;
// use std::thread::sleep;
// use std::time::Duration;

use crate::bytecode::Bytecode;
use crate::compiler::{NativeFunctionId};
use crate::uni_type::{UniResult, UniResultError};

#[derive(Debug, thiserror::Error)]
pub enum VMError {
	#[error("{0}")]
	UniResultError(#[from] UniResultError),
	#[error("{0}")]
	InvalidUTF8(#[from] FromUtf8Error),
	#[error("{0}")]
	Fmt(#[from] std::fmt::Error),
	#[error("Too few arguments for native function with id {id} call, expected {expected}, found {found}")]
	TooFewArgumentsForNativeFunctionCall {id: NativeFunctionId, found: u8, expected: u8 },
	#[error("Stack empty, but expected value")]
	StackEmptyButExpectedValue,
	#[error("Invalid stack position {0}")]
	InvalidStackPosition(u32),
}

pub struct VM {
	pc: u32,
	pub bytecode: Option<Vec<Bytecode>>,
	stack: Vec<UniResult>,
	ret_stack: Vec<u32>,
}

impl VM {
	pub fn new(bytecode: Option<Vec<Bytecode>>) -> Self {
		Self {pc: 0, bytecode, stack: Vec::new(), ret_stack: Vec::new()}
	}

	fn print(args: Vec<UniResult>) -> UniResult {
		for arg in args {
			print!("{arg} ");
		}

		println!();

		UniResult::None
	}
	fn tuple(args: Vec<UniResult>) -> UniResult {
		UniResult::Tuple(args)
	}
	fn range(args: &[UniResult]) -> Result<UniResult, VMError> {
		if args.len() == 1 {
			Err(VMError::TooFewArgumentsForNativeFunctionCall { id: NativeFunctionId::Range, found: 1, expected: 2 })
		} else {
			let start = &args[0];
			let end = &args[1];

			match (start, end) {
				(UniResult::Float(start), UniResult::Float(end)) => Ok(UniResult::Range { start: *start, end: *end, step: 1.0 }),
				_ => Err(VMError::UniResultError(UniResultError::IncompatibleOperationType))
			}
		}
	}

	fn call_native(&mut self, id: NativeFunctionId) -> Result<UniResult, VMError> {
		let mut args = Vec::new();

		let mut i = 0;

		while let Some(value) = self.stack.pop() && i < id.args() {
			args.push(value);

			i += 1;
		}

		match id {
			NativeFunctionId::Print => Ok(Self::print(args)),
			NativeFunctionId::Tuple => Ok(Self::tuple(args)),
			NativeFunctionId::Range => Self::range(&args),
		}
	}

	fn dump_stack(&self) {
		print!("stack = [");

		for (i, value) in self.stack.iter().enumerate() {
			if i > 0 {
				print!(", ");
			}

			print!("{value}");
		}

		println!("]");
	}

	fn exec_inst(&mut self) -> Result<(bool, bool), VMError> {
		let Some(bytecode) = &self.bytecode else {
			return Ok((false, false))
		};

		let Some(inst) = bytecode.get(self.pc as usize) else {
			return Ok((false, false))
		};

		let mut pc_changed = false;

		match inst {
			Bytecode::Halt => return Ok((false, false)),
			&Bytecode::PushBoolean(val) => {
				self.stack.push(val.into());
			},
			&Bytecode::PushFloat(val) => {
				self.stack.push(val.into());
			},
			Bytecode::PushString(val) => {
				self.stack.push(val.clone().into());
			},
			&Bytecode::Look(stack_pos) => {
				let usize_stack_pos = self.stack.len() - (stack_pos as usize) - 1;

				let val = self.stack.get(usize_stack_pos).ok_or(VMError::InvalidStackPosition(stack_pos))?;

				self.stack.push(val.clone());
			},
			&Bytecode::Load(stack_pos) => {
				let usize_stack_pos = self.stack.len() - (stack_pos as usize) - 1;

				let value = self.stack.pop().ok_or(VMError::StackEmptyButExpectedValue)?;

				if let Some(val) = self.stack.get_mut(usize_stack_pos) {
					*val = value;
				} else {
					return Err(VMError::InvalidStackPosition(stack_pos));
				}
			},
			Bytecode::Remove => {
				self.stack.pop().ok_or(VMError::StackEmptyButExpectedValue)?;
			},
			&Bytecode::Jif(dest_pc) => {
				if !self.stack.pop().ok_or(VMError::StackEmptyButExpectedValue)?.to_bool()? {
					self.pc = dest_pc;

					pc_changed = true;
				}
			},
			&Bytecode::Jit(dest_pc) => {
				if self.stack.pop().ok_or(VMError::StackEmptyButExpectedValue)?.to_bool()? {
					self.pc = dest_pc;

					pc_changed = true;
				}
			},
			&Bytecode::Jmp(dest_pc) => {
				self.pc = dest_pc;

				pc_changed = true;
			},
			&Bytecode::Call(dest_pc) => {
				self.stack.push((self.pc + 1).into());

				self.pc = dest_pc;

				pc_changed = true;
			},
			&Bytecode::Subprogram => {
				let pc = self.stack.pop().ok_or(VMError::StackEmptyButExpectedValue)?.to_u32()?;

				self.ret_stack.push(pc);
			},
			&Bytecode::Ret => {
				self.pc = self.ret_stack.pop().unwrap();

				pc_changed = true;
			},
			&Bytecode::NativeCall(id) => {
				let result = self.call_native(id)?;

				self.stack.push(result);
			},
			Bytecode::Eq |
			Bytecode::Neq |
			Bytecode::Lt |
			Bytecode::Gt |
			Bytecode::Add |
			Bytecode::Sub |
			Bytecode::Mul |
			Bytecode::Pow |
			Bytecode::Div |
			Bytecode::LogicalAnd |
			Bytecode::LogicalOr => {
				let right = self.stack.pop().ok_or(VMError::StackEmptyButExpectedValue)?;
				let left = self.stack.pop().ok_or(VMError::StackEmptyButExpectedValue)?;

				let result = left.calc_binary_by_op(&right, &inst.to_binary_op().unwrap())?;

				self.stack.push(result);
			},
			Bytecode::Neg |
			Bytecode::Not => {
				let value = self.stack.pop().ok_or(VMError::StackEmptyButExpectedValue)?;

				let result = value.calc_unary_by_op(&inst.to_unary_op().unwrap())?;

				self.stack.push(result);
			},
		}

		Ok((true, pc_changed))
	}

	pub fn exec(&mut self) -> Result<Option<UniResult>, VMError> {
		self.pc = 0;

		while let (is_not_hlt, pc_changed) = self.exec_inst()? && is_not_hlt {
			self.dump_stack();

			if !pc_changed {
				self.pc += 1;
			}

			// sleep(Duration::from_millis(100));
		}

		Ok(self.stack.pop())
	}
}
