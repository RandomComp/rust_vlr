use std::{collections::VecDeque, fmt::Display, iter::zip, string::FromUtf8Error};

use crate::compiler::{Bytecode, Compiler};

#[derive(Debug, thiserror::Error)]
pub enum VMError {
	#[error("incompatible operation")]
	IncompatibleOperationType,
	#[error("invalid UTF-8")]
	InvalidUTF8(#[from] FromUtf8Error),
}

#[derive(Debug, Clone, PartialEq)]
pub enum VMResult {
	ArgsEnd,
	None,
	Number(f64),
	String(String),
	Tuple(Vec<VMResult>),
}

impl Display for VMResult {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			VMResult::ArgsEnd => write!(f, "<args end>"),
			VMResult::None => write!(f, "None"),
			VMResult::Number(x) => write!(f, "{x}"),
			VMResult::String(x) => write!(f, "{x}"),
			VMResult::Tuple(values) => {
				write!(f, "(")?;

				for (i, value) in values.iter().enumerate() {
					if i > 0 {
						write!(f, ", ")?;
					}

					write!(f, "{value}")?;
				}

				write!(f, ")")?;

				Ok(())
			},
		}
	}
}

impl VMResult {
	fn add(&self, right: &Self) -> Result<Self, VMError> {
		match (self, right) {
			(VMResult::Number(a), &VMResult::Number(b))
				=> Ok(VMResult::Number(a + b)),
			(VMResult::Tuple(values), b @ VMResult::Number(..)) |
			(b @ VMResult::Number(..), VMResult::Tuple(values)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(value.add(b)?);
				}

				Ok(result.into())
			},
			(VMResult::Tuple(values_a), VMResult::Tuple(values_b)) => {
				let mut result = Vec::new();

				for (a, b) in zip(values_a, values_b) {
					result.push(a.add(b)?);
				}

				Ok(result.into())
			},
			_ => Err(VMError::IncompatibleOperationType),
		}
	}
	fn sub(self, right: &Self) -> Result<Self, VMError> {
		match (self, right) {
			(VMResult::Number(a), &VMResult::Number(b))
				=> Ok(VMResult::Number(a - b)),
			(VMResult::Tuple(values), b @ VMResult::Number(..)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(value.sub(b)?);
				}

				Ok(result.into())
			},
			(b @ VMResult::Number(..), VMResult::Tuple(values)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(b.clone().sub(value)?);
				}

				Ok(result.into())
			},
			(VMResult::Tuple(values_a), VMResult::Tuple(values_b)) => {
				let mut result = Vec::new();

				for (a, b) in zip(values_a, values_b) {
					result.push(a.sub(b)?);
				}

				Ok(result.into())
			},
			_ => Err(VMError::IncompatibleOperationType),
		}
	}
	fn mul(&self, right: &Self) -> Result<Self, VMError> {
		match (self, right) {
			(VMResult::Number(a), VMResult::Number(b))
				=> Ok(VMResult::Number(a * b)),
			(VMResult::Tuple(values), &VMResult::Number(b)) |
			(&VMResult::Number(b), VMResult::Tuple(values)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(value.mul(&b.into())?);
				}

				Ok(result.into())
			},

			(VMResult::Tuple(values_a), VMResult::Tuple(values_b)) => {
				let mut result = Vec::new();

				for (a, b) in zip(values_a, values_b) {
					result.push(a.mul(b)?);
				}

				Ok(result.into())
			},
			_ => Err(VMError::IncompatibleOperationType),
		}
	}
	fn div(&self, right: &Self) -> Result<Self, VMError> {
		match (self, right) {
			(VMResult::Number(a), &VMResult::Number(b))
				=> Ok(VMResult::Number(a / b)),
			(VMResult::Tuple(values), b @ VMResult::Number(..)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(value.div(b)?);
				}

				Ok(result.into())
			},
			(b @ VMResult::Number(..), VMResult::Tuple(values)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(b.clone().div(value)?);
				}

				Ok(result.into())
			},
			(VMResult::Tuple(values_a), VMResult::Tuple(values_b)) => {
				let mut result = Vec::new();

				for (a, b) in zip(values_a, values_b) {
					result.push(a.div(b)?);
				}

				Ok(result.into())
			},
			_ => Err(VMError::IncompatibleOperationType),
		}
	}
	fn pow(&self, right: &Self) -> Result<Self, VMError> {
		match (self, right) {
			(VMResult::Number(a), &VMResult::Number(b))
				=> Ok(VMResult::Number(a.powf(b))),
			(VMResult::Tuple(values), b @ VMResult::Number(..)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(value.pow(b)?);
				}

				Ok(result.into())
			},
			(b @ VMResult::Number(..), VMResult::Tuple(values)) => {
				let mut result = Vec::new();

				for value in values {
					result.push(b.clone().pow(value)?);
				}

				Ok(result.into())
			},
			(VMResult::Tuple(values_a), VMResult::Tuple(values_b)) => {
				let mut result = Vec::new();

				for (a, b) in zip(values_a, values_b) {
					result.push(a.pow(b)?);
				}

				Ok(result.into())
			},
			_ => Err(VMError::IncompatibleOperationType),
		}
	}
}

impl From<f64> for VMResult {
	fn from(value: f64) -> Self {
		VMResult::Number(value)
	}
}

impl From<String> for VMResult {
	fn from(value: String) -> Self {
		VMResult::String(value)
	}
}

impl From<Vec<VMResult>> for VMResult {
	fn from(values: Vec<VMResult>) -> Self {
		VMResult::Tuple(values)
	}
}

impl From<VMResult> for f64 {
	fn from(value: VMResult) -> f64 {
		if let VMResult::Number(x) = value {
			x
		} else {
			panic!("Using 'into' on incompatible type {value:?}")
		}
	}
}

impl From<VMResult> for String {
	fn from(value: VMResult) -> String {
		if let VMResult::String(x) = value {
			x
		} else {
			panic!("Using 'into' on incompatible type {value:?}")
		}
	}
}

impl From<VMResult> for Vec<VMResult> {
	fn from(value: VMResult) -> Vec<VMResult> {
		if let VMResult::Tuple(x) = value {
			x
		} else {
			panic!("Using 'into' on incompatible type {value:?}")
		}
	}
}

pub struct VM {
	pc: usize,
	pub bytecode: Option<Vec<u8>>,
	stack: VecDeque<VMResult>,
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

	fn print(args: Vec<VMResult>) -> VMResult {
		for arg in args {
			println!("{arg}");
		}

		VMResult::None
	}

	fn pop_stack_until_value(&mut self, value: &VMResult) -> Vec<VMResult> {
		let mut result = Vec::new();

		while let Some(x) = self.stack.pop_back() && &x != value {
			result.push(x);
		}

		result
	}

	fn disasm_inst(&mut self) -> Result<bool, VMError> {
		let result = match self.peek(0) {
			Some(Bytecode::NOP | Bytecode::NOP_FF) => true,
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
			Some(Bytecode::MAKE_TUPLE) => {
				self.consume_next(1);

				println!("make_tuple");

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
			Some(Bytecode::NATIVE_CALL) => {
				self.consume_next(1);

				let id = u8::from_le_bytes(self.consume_const_bytes_and_get::<1>());

				println!("native_call {}", Compiler::get_native_call_name_by_id(id).unwrap());

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

	fn exec_inst(&mut self) -> Result<bool, VMError> {
		self.disasm_inst_nochange()?;

		let result = match self.peek(0) {
			Some(Bytecode::NOP | Bytecode::NOP_FF) => true,
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
			Some(Bytecode::PUSH_ARG_END) => {
				self.consume_next(1);

				self.stack.push_back(VMResult::ArgsEnd);

				true
			},
			Some(Bytecode::MAKE_TUPLE) => {
				self.consume_next(1);

				let values = self.pop_stack_until_value(&VMResult::ArgsEnd);

				self.stack.push_back(VMResult::Tuple(values));

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
			Some(Bytecode::NATIVE_CALL) => {
				self.consume_next(1);

				let id = u8::from_le_bytes(self.consume_const_bytes_and_get::<1>());

				if id == 0 {
					let args = self.pop_stack_until_value(&VMResult::ArgsEnd);

					self.stack.push_back(Self::print(args));
				}

				true
			}
			Some(x) => panic!("invalid instruction byte 0x{x:02X} at {}", self.pc),
			None => false,
		};

		Ok(result)
	}

	pub fn exec(&mut self) -> Result<Option<VMResult>, VMError> {
		self.pc = 0;

		while self.exec_inst()? {
			println!("stack = {:?}", self.stack);
		}

		Ok(self.stack.pop_back())
	}
}
