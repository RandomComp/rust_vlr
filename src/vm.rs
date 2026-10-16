use std::{fmt};
use std::string::FromUtf8Error;
// use std::thread::sleep;
// use std::time::Duration;

use crate::bytecode::{Bytecode, VMRegs};
use crate::compiler::parser::BinaryOp;
use crate::native_func::{NativeFunctionArgsNumber, NativeFunctionID};
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

type Stack = Vec<UniResult>;

pub struct VM {
	pub bytecode: Option<Vec<Bytecode>>,
	stack: Stack,

	regs: VMRegs,
}

impl VM {
	pub fn new(bytecode: Option<Vec<Bytecode>>) -> Self {
		Self {
			bytecode,
			stack: Vec::new(),

			regs: VMRegs::default(),
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
					let Some(value) = self.stack.pop() else {
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
			NativeFunctionArgsNumber::Dynamic => {
				if let Some(fixed) = self.stack.pop().map(|v| i32::try_from(&v)) {
					let fixed = fixed?.cast_unsigned();

					while i < fixed  {
						let Some(value) = self.stack.pop() else {
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
		println!("{}", self.regs);
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

	fn exec_inst(&mut self) -> Result<(bool, bool), RawVMError> {
		let Some(bytecode) = &self.bytecode else {
			return Ok((false, false))
		};

		let Some(inst) = bytecode.get(self.regs.pc as usize) else {
			return Ok((false, false))
		};

		let mut pc_changed = false;

		println!("{inst}");

		match inst {
			Bytecode::Halt =>
				return Ok((false, false)),
			&Bytecode::PushNumber(val) => {
				self.stack.push(val.into());
			},
			&Bytecode::PushFloat(val) => {
				self.stack.push(val.into());
			},
			Bytecode::PushString(val) => {
				self.stack.push(val.clone().into());
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

				self.stack.push(value.into());
			},
			&Bytecode::PopReg(reg) => {
				let value = u32::try_from(&self.stack.pop().ok_or(RawVMError::StackEmptyButExpectedValue)?)?;

				self.regs[reg] = value;
			},

			&Bytecode::Look(stack_pos) => {
				let usize_stack_pos = stack_pos as usize;

				let val = self.stack.get(usize_stack_pos).ok_or(RawVMError::InvalidStackPosition(usize_stack_pos))?;

				self.stack.push(val.clone());
			},
			&Bytecode::Load(stack_pos) => {
				let usize_stack_pos = stack_pos as usize;

				let value = self.stack.pop().ok_or(RawVMError::StackEmptyButExpectedValue)?;

				if let Some(val) = self.stack.get_mut(usize_stack_pos) {
					*val = value;
				} else {
					return Err(RawVMError::InvalidStackPosition(usize_stack_pos));
				}
			},
			Bytecode::Remove => {
				self.stack.pop().ok_or(RawVMError::StackEmptyButExpectedValue)?;
			},
			&Bytecode::Jif(dest_pc) => {
				if !bool::try_from(self.stack.pop().ok_or(RawVMError::StackEmptyButExpectedValue)?)? {
					self.regs.pc = dest_pc;

					pc_changed = true;
				}
			},
			&Bytecode::Jmp(dest_pc) => {
				self.regs.pc = dest_pc;

				pc_changed = true;
			},
			&Bytecode::Call(dest_pc) => {
				self.stack.push((self.regs.pc + 1).into());

				self.regs.pc = dest_pc;

				pc_changed = true;
			},
			&Bytecode::Ret => {
				self.regs.pc = u32::try_from(&self.stack.pop().ok_or(RawVMError::StackEmptyButExpectedValue)?)?;

				pc_changed = true;
			},
			&Bytecode::NativeCall(id) => {
				let result = self.call_native(id)?;

				// self.regs.acc = result;
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
				let right = self.stack.pop()
								.ok_or(RawVMError::StackEmptyButExpectedValue)?;

				let left = self.stack.pop()
								.ok_or(RawVMError::StackEmptyButExpectedValue)?;

				let op = &inst.to_binary_op().unwrap_or_else(|| unreachable!("invalid binary operation {inst} in match"));

				let result = UniResult::calc_binary_by_op(
					&Spanned::from_with_span(left, 0),
					&Spanned::from_with_span(right, 0),
					op
				).map(|v| v.val).map_err(|e| e.val)?;

				self.stack.push(result);
			},
			Bytecode::Neg |
			Bytecode::Not => {
				let value = self.stack.pop().ok_or(RawVMError::StackEmptyButExpectedValue)?;

				let op = &inst.to_unary_op().unwrap_or_else(|| unreachable!("invalid unary operation {inst} in match"));

				let result = UniResult::calc_unary_by_op(
					&Spanned::from_with_span(value, 0),
					op
				).map(|v| v.val).map_err(|e| e.val)?;

				self.stack.push(result);
			},
		}

		Ok((true, pc_changed))
	}

	pub fn exec(&mut self) -> Result<Option<UniResult>, VMError> {
		self.regs.pc = 0;

		loop {
			let (is_not_hlt, pc_changed) = match self.exec_inst() {
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

					let Some(inst) = bytecode.get(self.regs.pc as usize) else {
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

			if !pc_changed {
				self.regs.pc += 1;
			}

			// sleep(Duration::from_millis(100));
		}

		Ok(self.stack.pop())
	}
}
