use std::{collections::VecDeque, ops::IndexMut, string::FromUtf8Error};

use crate::{compiler::{Bytecode, NativeFunctionId}, uni_result::{UniResult, UniResultError}};

#[derive(Debug, thiserror::Error)]
pub enum VMError {
	#[error("UniResult error: {0}")]
	UniResultError(#[from] UniResultError),
	#[error("invalid UTF-8: {0}")]
	InvalidUTF8(#[from] FromUtf8Error),
	// #[error("Unknown native function with id {0} call")]
	// UnknownNativeFunctionCall(NativeFunctionId),
	#[error("Too few arguments for native function with id {id} call, expected {expected}, found {found}")]
	TooFewArgumentsForNativeFunctionCall {id: NativeFunctionId, found: u8, expected: u8 },
}

pub struct VM {
	pc: usize,
	pub bytecode: Option<Vec<u8>>,
	stack: VecDeque<UniResult>,
}

impl VM {
	pub fn new(bytecode: Option<Vec<u8>>) -> Self {
		Self {pc: 0, bytecode, stack: VecDeque::new()}
	}

	fn peek(&mut self, offset: usize) -> Option<u8> {
		self.bytecode.as_mut().and_then(|v| v.get(self.pc + offset).copied())
	}

	fn consume_next(&mut self, len: usize) -> Option<u8> {
		let result = self.peek(0);

		self.pc += len;

		result
	}

	fn consume_const_bytes_and_get<const LEN: usize>(&mut self) -> [u8; LEN] {
		let mut result = [0u8; LEN];

		for i in 0..LEN {
			if let Some(x) = self.peek(i) {
				result[i] = x;
			} else {
				break
			}
		}

		self.pc += LEN;

		result
	}

	fn consume_bytes_and_get(&mut self, len: usize) -> Vec<u8> {
		let mut result = Vec::with_capacity(len);

		for i in 0..len {
			if let Some(x) = self.peek(i) {
				result.push(x);
			} else {
				break
			}
		}

		self.pc += len;

		result
	}

	fn pop_stack_until_value(&mut self, value: &UniResult) -> Vec<UniResult> {
		let mut result = Vec::new();

		while let Some(x) = self.stack.pop_back() && &x != value {
			result.push(x);
		}

		result
	}

	fn disasm_inst(&mut self) -> Result<bool, VMError> {
		println!("{:02}:", self.pc);

		let result = match self.peek(0) {
			Some(Bytecode::NOP | Bytecode::NOP_FF) => true,
			Some(Bytecode::PUSH_FALSE) => {
				self.consume_next(1);

				println!("push false");

				true
			}
			Some(Bytecode::PUSH_TRUE) => {
				self.consume_next(1);

				println!("push true");

				true
			}
			Some(Bytecode::PUSH_NUMBER) => {
				self.consume_next(1);

				let bytes = self.consume_const_bytes_and_get::<8>();

				let number = f64::from_le_bytes(bytes);

				println!("push {number}");

				true
			},
			Some(Bytecode::PUSH_STRING) => {
				self.consume_next(1);

				let len = u16::from_le_bytes(self.consume_const_bytes_and_get::<2>()) as usize;

				let bytes = self.consume_bytes_and_get(len);

				let text = String::from_utf8(bytes)?;

				println!("push \"{text}\"");

				true
			},
			Some(Bytecode::LOOK) => {
				self.consume_next(1);

				let index = u32::from_le_bytes(self.consume_const_bytes_and_get::<4>()) as usize;

				println!("look {index}");

				true
			},
			Some(Bytecode::LOAD) => {
				self.consume_next(1);

				let index = u32::from_le_bytes(self.consume_const_bytes_and_get::<4>()) as usize;

				println!("load {index}");

				true
			},
			Some(Bytecode::REMOVE) => {
				self.consume_next(1);

				println!("remove");

				true
			},
			Some(Bytecode::PUSH_ARG_END) => {
				self.consume_next(1);

				println!("push args_end");

				true
			},
			Some(Bytecode::ADD) => {
				self.consume_next(1);

				println!("add");

				true
			},
			Some(Bytecode::SUB) => {
				self.consume_next(1);

				println!("sub");

				true
			},
			Some(Bytecode::MUL) => {
				self.consume_next(1);

				println!("mul");

				true
			},
			Some(Bytecode::DIV) => {
				self.consume_next(1);

				println!("div");

				true
			},
			Some(Bytecode::POW) => {
				self.consume_next(1);

				println!("pow");

				true
			},
			Some(Bytecode::EQ) => {
				self.consume_next(1);

				println!("eq");

				true
			},
			Some(Bytecode::NEQ) => {
				self.consume_next(1);

				println!("neq");

				true
			},
			Some(Bytecode::LT) => {
				self.consume_next(1);

				println!("lt");

				true
			},
			Some(Bytecode::GT) => {
				self.consume_next(1);

				println!("gt");

				true
			},
			Some(Bytecode::LOG_AND) => {
				self.consume_next(1);

				println!("log_and");

				true
			},
			Some(Bytecode::LOG_OR) => {
				self.consume_next(1);

				println!("log_or");

				true
			},
			Some(Bytecode::NEG) => {
				self.consume_next(1);

				println!("neg");

				true
			},
			Some(Bytecode::NOT) => {
				self.consume_next(1);

				println!("not");

				true
			},
			Some(Bytecode::JIT) => {
				self.consume_next(1);

				let jump = i32::from_le_bytes(self.consume_const_bytes_and_get::<4>()) + 5;

				println!("jit +{jump:02}");

				true
			},
			Some(Bytecode::JIF) => {
				self.consume_next(1);

				let jump = i32::from_le_bytes(self.consume_const_bytes_and_get::<4>()) + 5;

				println!("jif +{jump:02}");

				true
			},
			Some(Bytecode::JMP) => {
				self.consume_next(1);

				let jump = i32::from_le_bytes(self.consume_const_bytes_and_get::<4>()) + 5;

				println!("jmp +{jump:02}");

				true
			},
			Some(Bytecode::NATIVE_CALL) => {
				self.consume_next(1);

				let id = u8::from_le_bytes(self.consume_const_bytes_and_get::<1>());

				println!("native_call {}", NativeFunctionId::from(id).get_name());

				true
			}
			Some(x) => panic!("invalid instruction byte 0x{x:02X} at {}", self.pc),
			None => false,
		};

		Ok(result)
	}

	fn disasm_inst_nochange(&mut self) -> Result<(), VMError> {
		let index = self.pc;

		self.disasm_inst()?;

		self.pc = index;

		Ok(())
	}

	pub fn disasm(&mut self) -> Result<(), VMError> {
		self.pc = 0;

		while self.disasm_inst()? {}

		Ok(())
	}

	fn print(args: Vec<UniResult>) -> UniResult {
		for arg in args {
			println!("{arg}");
		}

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
				(UniResult::Number(start), UniResult::Number(end)) => Ok(UniResult::Range { start: *start, end: *end, step: 1.0 }),
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

	fn exec_inst(&mut self) -> Result<bool, VMError> {
		self.disasm_inst_nochange()?;

		let result = match self.peek(0) {
			Some(Bytecode::PUSH_FALSE) => {
				self.consume_next(1);

				self.stack.push_back(false.into());

				true
			},
			Some(Bytecode::PUSH_TRUE) => {
				self.consume_next(1);

				self.stack.push_back(true.into());

				true
			},
			Some(Bytecode::PUSH_NUMBER) => {
				self.consume_next(1);

				let bytes = self.consume_const_bytes_and_get::<8>();

				let number = f64::from_le_bytes(bytes);

				self.stack.push_back(number.into());

				true
			},
			Some(Bytecode::PUSH_STRING) => {
				self.consume_next(1);

				let len = u16::from_le_bytes(self.consume_const_bytes_and_get::<2>()) as usize;

				let bytes = self.consume_bytes_and_get(len);

				let text = String::from_utf8(bytes)?;

				self.stack.push_back(text.into());

				true
			},
			Some(Bytecode::LOOK) => {
				self.consume_next(1);

				let index = u32::from_le_bytes(self.consume_const_bytes_and_get::<4>()) as usize;

				if let Some(value) = self.stack.get(index) {
					self.stack.push_back(value.clone());
				}

				true
			},
			Some(Bytecode::LOAD) => {
				self.consume_next(1);

				let index = u32::from_le_bytes(self.consume_const_bytes_and_get::<4>()) as usize;

				if index < self.stack.len() {
					let value = self.stack.back().unwrap();

					*self.stack.index_mut(index) = value.clone();
				}

				true
			},
			Some(Bytecode::REMOVE) => {
				self.consume_next(1);

				self.stack.pop_back();

				true
			},
			Some(Bytecode::PUSH_ARG_END) => {
				self.consume_next(1);

				self.stack.push_back(UniResult::ArgsEnd);

				true
			},
			Some(Bytecode::ADD) => {
				self.consume_next(1);

				let left = self.stack.pop_back().unwrap();
				let right = self.stack.pop_back().unwrap();

				self.stack.push_back(left.add(&right)?);

				true
			},
			Some(Bytecode::SUB) => {
				self.consume_next(1);

				let left = self.stack.pop_back().unwrap();
				let right = self.stack.pop_back().unwrap();

				self.stack.push_back(left.sub(&right)?);

				true
			},
			Some(Bytecode::MUL) => {
				self.consume_next(1);

				let left = self.stack.pop_back().unwrap();
				let right = self.stack.pop_back().unwrap();

				self.stack.push_back(left.mul(&right)?);

				true
			},
			Some(Bytecode::DIV) => {
				self.consume_next(1);

				let left = self.stack.pop_back().unwrap();
				let right = self.stack.pop_back().unwrap();

				self.stack.push_back(left.div(&right)?);

				true
			},
			Some(Bytecode::POW) => {
				self.consume_next(1);

				let left = self.stack.pop_back().unwrap();
				let right = self.stack.pop_back().unwrap();

				self.stack.push_back(left.pow(&right)?);

				true
			},
			Some(Bytecode::EQ) => {
				self.consume_next(1);

				let left = self.stack.pop_back().unwrap();
				let right = self.stack.pop_back().unwrap();

				self.stack.push_back(left.eq(&right)?);

				true
			},
			Some(Bytecode::NEQ) => {
				self.consume_next(1);

				let left = self.stack.pop_back().unwrap();
				let right = self.stack.pop_back().unwrap();

				self.stack.push_back(left.neq(&right)?);

				true
			},
			Some(Bytecode::LT) => {
				self.consume_next(1);

				let left = self.stack.pop_back().unwrap();
				let right = self.stack.pop_back().unwrap();

				self.stack.push_back(left.lt(&right)?);

				true
			},
			Some(Bytecode::GT) => {
				self.consume_next(1);

				let left = self.stack.pop_back().unwrap();
				let right = self.stack.pop_back().unwrap();

				self.stack.push_back(left.gt(&right)?);

				true
			},
			Some(Bytecode::LOG_AND) => {
				self.consume_next(1);

				let left = self.stack.pop_back().unwrap();
				let right = self.stack.pop_back().unwrap();

				self.stack.push_back(left.log_and(&right)?);

				true
			},
			Some(Bytecode::LOG_OR) => {
				self.consume_next(1);

				let left = self.stack.pop_back().unwrap();
				let right = self.stack.pop_back().unwrap();

				self.stack.push_back(left.log_or(&right)?);

				true
			},
			Some(Bytecode::NEG) => {
				self.consume_next(1);

				if let Some(value) = self.stack.pop_back() {
					self.stack.push_back(value.neg()?);
				}

				true
			},
			Some(Bytecode::NOT) => {
				self.consume_next(1);

				if let Some(value) = self.stack.pop_back() {
					self.stack.push_back(value.not()?);
				}

				true
			},
			Some(Bytecode::JIT) => {
				self.consume_next(1);

				let jump = i32::from_le_bytes(self.consume_const_bytes_and_get::<4>());

				if self.stack.pop_back().expect("Expected condition in stack (boolean value)").to_bool()? {
					if jump < 0 {
						self.pc -= jump.unsigned_abs() as usize;
					} else {
						self.pc += jump.unsigned_abs() as usize;
					}
				}

				true
			},
			Some(Bytecode::JIF) => {
				self.consume_next(1);

				let jump = i32::from_le_bytes(self.consume_const_bytes_and_get::<4>());

				if !self.stack.pop_back().expect("Expected condition in stack (boolean value)").to_bool()? {
					if jump < 0 {
						self.pc -= jump.unsigned_abs() as usize;
					} else {
						self.pc += jump.unsigned_abs() as usize;
					}
				}

				true
			},
			Some(Bytecode::JMP) => {
				self.consume_next(1);

				let jump = i32::from_le_bytes(self.consume_const_bytes_and_get::<4>());

				if jump < 0 {
					self.pc -= jump.unsigned_abs() as usize;
				} else {
					self.pc += jump.unsigned_abs() as usize;
				}

				true
			},
			Some(Bytecode::NATIVE_CALL) => {
				self.consume_next(1);

				let id = u8::from_le_bytes(self.consume_const_bytes_and_get::<1>());

				let args = self.pop_stack_until_value(&UniResult::ArgsEnd);

				match Self::call_native(id.into(), args)? {
					UniResult::None => {},
					val => self.stack.push_back(val),
				}

				true
			}
			Some(Bytecode::NOP | Bytecode::NOP_FF) | None => false,
			Some(x) => panic!("invalid instruction byte 0x{x:02X} at {:02X}", self.pc),
		};

		Ok(result)
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

		while self.exec_inst()? {
			self.dump_stack();
		}

		Ok(self.stack.pop_back())
	}
}
