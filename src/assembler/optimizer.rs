use crate::{assembler::{Bytecode, BytecodeRegister}, compiler::native_func::NativeFunctionArgsNumber};

#[derive(Debug, Clone, PartialEq)]
enum StackValue {
	Number(u32),
	Reg(BytecodeRegister),
}

impl From<&Bytecode> for StackValue {
	fn from(value: &Bytecode) -> Self {
		match *value {
			Bytecode::PushNumber(num) =>
				Self::Number(num),
			Bytecode::PushReg(reg) =>
				Self::Reg(reg),
			_ => todo!("{value}"),
		}
	}
}

#[derive(Debug)]
pub struct AssemblerOptimizer<'a> {
	stack: Vec<(usize, StackValue)>,
	affected_regs: Vec<(BytecodeRegister, bool)>,
	bytes: Option<&'a [Bytecode]>,
	index: usize,
}

impl<'a> AssemblerOptimizer<'a> {
	pub fn new(bytes: &'a [Bytecode]) -> Self {
		Self { stack: Vec::new(), affected_regs: Vec::new(), bytes: Some(bytes), index: 0 }
	}

	fn peek(bytes: Option<&'a [Bytecode]>, index: usize) -> Option<&'a Bytecode> {
		bytes?.get(index)
	}

	fn consumed_inst(&mut self) {
		self.index += 1;
	}

	fn next_inst(&mut self, result: &mut Vec<Bytecode>) {
		let Some(cur) = Self::peek(self.bytes, self.index) else {
			return
		};

		result.push(cur.clone());

		self.consumed_inst();
	}

	pub fn optimize(&mut self) -> Vec<Bytecode> {
		let mut result = Vec::new();

		while let Some(cur) = Self::peek(self.bytes, self.index) {
			match *cur {
				Bytecode::PushReg(reg) => {
					self.stack.push((result.len(), cur.into()));

					self.affected_regs.push((reg, false));

					self.next_inst(&mut result);
				}
				Bytecode::Add | Bytecode::Sub | Bytecode::Mul | Bytecode::Div |
			 	Bytecode::Eq | Bytecode::Lt | Bytecode::Neg | Bytecode::Not => {
					if let Some((reg2, is_affected)) = self.affected_regs.last_mut() && *reg2 == BytecodeRegister::Accum {
						*is_affected = true;
					}

					self.next_inst(&mut result);
				}
				Bytecode::MovU32ToReg(reg, _) | Bytecode::MovRegToReg(reg, _) => {
					if let Some((reg2, is_affected)) = self.affected_regs.last_mut() && reg == *reg2 {
						*is_affected = true;
					}

					self.next_inst(&mut result);
				}
				Bytecode::PushNumber(_) => {
					self.stack.push((result.len(), cur.into()));

					self.next_inst(&mut result);
				}
				Bytecode::PopReg(reg) => {
					let Some((index, val)) = self.stack.pop() else {
						self.next_inst(&mut result);

						continue;
					};

					match val {
						StackValue::Number(val) => {
							result.push(Bytecode::MovU32ToReg(reg, val));

							self.consumed_inst();

							result[index] = Bytecode::Nop;
						},
						StackValue::Reg(reg1) => {
							let Some((affected_reg, is_affected)) = self.affected_regs.last_mut() else {
								self.next_inst(&mut result);

								continue;
							};

							if reg1 == *affected_reg {
								if !*is_affected {
									if reg != reg1 {
										result.push(Bytecode::MovRegToReg(reg, reg1));
									}

									self.consumed_inst();

									result[index] = Bytecode::Nop;
								}

								self.affected_regs.pop();

								self.next_inst(&mut result);
							} else {
								*is_affected = true;

								println!("*is_affected = true;");

								self.next_inst(&mut result);
							}
						}
					}
				}
				Bytecode::NativeCall(id) => {
					let expected = id.args();

					match expected {
						NativeFunctionArgsNumber::Fixed(fixed) => {
							for _ in 0..fixed {
								self.stack.pop();
							}
						}
						NativeFunctionArgsNumber::Dynamic => {
							let Some((_, StackValue::Number(fixed))) = self.stack.pop() else {
								self.next_inst(&mut result);

								continue;
							};

							for _ in 0..fixed {
								self.stack.pop();
							}
						}
					}

					self.next_inst(&mut result);
				}
				_ => self.next_inst(&mut result),
			}
		}

		result.retain(|v| *v != Bytecode::Nop);

		result
	}
}
