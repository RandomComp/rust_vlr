use std::{collections::VecDeque, string::FromUtf8Error};

use crate::{compiler::{Bytecode, NativeFunctionId}, uni_type::{UniResult, UniResultError}};

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
	stack: VecDeque<UniResult>,
}

impl VM {
	pub fn new(bytecode: Option<Vec<Bytecode>>) -> Self {
		Self {pc: 0, bytecode, stack: VecDeque::new()}
	}

	fn pop_stack_until_value(stack: &mut VecDeque<UniResult>, value: &UniResult) -> Vec<UniResult> {
		let mut result = Vec::new();

		while let Some(x) = stack.pop_back() && &x != value {
			result.push(x);
		}

		result
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

	fn call_native(id: NativeFunctionId, args: Vec<UniResult>) -> Result<UniResult, VMError> {
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

	pub fn exec(&mut self) -> Result<Option<UniResult>, VMError> {
		self.pc = 0;

		let Some(bytecode) = &self.bytecode else {
			return Ok(None)
		};

		for inst in bytecode {
			match inst {
				Bytecode::Halt => break,
				Bytecode::PushArgEnd =>
					self.stack.push_back(UniResult::ArgsEnd),
				&Bytecode::PushBoolean(val) => {
					self.stack.push_back(val.into());
				},
				&Bytecode::PushFloat(val) => {
					self.stack.push_back(val.into());
				},
				Bytecode::PushString(val) => {
					self.stack.push_back(val.clone().into());
				},
				&Bytecode::Look(stack_pos) => {
					let val = self.stack.get(stack_pos as usize).ok_or(VMError::InvalidStackPosition(stack_pos))?;

					self.stack.push_back(val.clone());
				},
				&Bytecode::Load(stack_pos) => {
					let value = self.stack.pop_back().ok_or(VMError::StackEmptyButExpectedValue)?;

					if let Some(val) = self.stack.get_mut(stack_pos as usize) {
						*val = value;
					} else {
						return Err(VMError::InvalidStackPosition(stack_pos));
					}
				},
				Bytecode::Remove => {
					return self.stack.pop_back().ok_or(VMError::StackEmptyButExpectedValue).map(Some)
				},
				&Bytecode::Jif(dest_pc) => {
					if !self.stack.pop_back().ok_or(VMError::StackEmptyButExpectedValue)?.to_bool()? {
						self.pc = dest_pc;
					}
				},
				&Bytecode::Jit(dest_pc) => {
					if self.stack.pop_back().ok_or(VMError::StackEmptyButExpectedValue)?.to_bool()? {
						self.pc = dest_pc;
					}
				},
				&Bytecode::Jmp(dest_pc) => {
					self.pc = dest_pc;
				},
				&Bytecode::Call(dest_pc) => {
					self.stack.push_back(self.pc.into());

					self.pc = dest_pc;
				},
				&Bytecode::Ret => {
					self.pc = self.stack.pop_back().ok_or(VMError::StackEmptyButExpectedValue)?.to_u32()?;
				},
				&Bytecode::NativeCall(id) => {
					let args = Self::pop_stack_until_value(&mut self.stack, &UniResult::ArgsEnd);

					Self::call_native(id, args)?;
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
					let left = self.stack.pop_back().ok_or(VMError::StackEmptyButExpectedValue)?;
					let right = self.stack.pop_back().ok_or(VMError::StackEmptyButExpectedValue)?;

					let result = left.calc_binary_by_op(&right, &inst.to_binary_op().unwrap())?;

					self.stack.push_back(result);
				},
				Bytecode::Neg |
				Bytecode::Not => {
					let value = self.stack.pop_back().ok_or(VMError::StackEmptyButExpectedValue)?;

					let result = value.calc_unary_by_op(&inst.to_unary_op().unwrap())?;

					self.stack.push_back(result);
				},
			}
		}

		Ok(self.stack.pop_back())
	}
}
