use crate::assembler::{Bytecode, native_func::NativeFunctionArgsNumber};

#[derive(Debug)]
pub struct AssemblerOptimizer<'a> {
	stack: Vec<u32>,
	bytes: Option<&'a [Bytecode]>,
	index: usize,
}

impl<'a> AssemblerOptimizer<'a> {
	pub fn new(bytes: &'a [Bytecode]) -> Self {
		Self { stack: Vec::new(), bytes: Some(bytes), index: 0 }
	}

	fn peek(bytes: Option<&'a [Bytecode]>, index: usize) -> Option<&Bytecode> {
		bytes?.get(index)
	}

	fn consume_next(&mut self) {
		self.index += 1;
	}

	fn optimize_inst(&mut self, result: &mut Vec<Bytecode>) -> bool {
		let Some(cur) = Self::peek(self.bytes, self.index + 0) else {
			return false
		};

		let Some(next) = Self::peek(self.bytes, self.index + 1) else {
			return false
		};

		match (cur, next) {
			(Bytecode::PushReg(reg1), Bytecode::PopReg(reg2)) if reg1 == reg2 => {
				true
			}
			(&Bytecode::PushReg(reg1), &Bytecode::PopReg(reg2)) => {
				result.push(Bytecode::MovRegToReg(reg2, reg1));

				true
			}
			(&Bytecode::PushNumber(num), &Bytecode::PopReg(reg)) => {
				result.push(Bytecode::MovU32ToReg(reg, num));

				true
			}
// 			(&Bytecode::PushNumber(num), _) => {
// 				self.stack.push(num);
//
// 				result.push(cur.clone());
//
// 				true
// 			}
// 			(&Bytecode::PopReg(reg), _) => {
// 				let Some(val) = self.stack.pop() else {
// 					return false
// 				};
//
// 				result.push(Bytecode::MovU32ToReg(reg, val));
//
// 				true
// 			}
// 			(&Bytecode::NativeCall(id), _) => {
// 				let expected = id.args();
//
// 				match expected {
// 					NativeFunctionArgsNumber::Fixed(fixed) => {
// 						for _ in 0..fixed {
// 							self.stack.pop();
// 						}
// 					}
// 					NativeFunctionArgsNumber::Dynamic => {
// 						let Some(fixed) = self.stack.pop() else {
// 							return false
// 						};
//
// 						for _ in 0..fixed {
// 							self.stack.pop();
// 						}
// 					}
// 				}
//
// 				true
// 			}
			_ => false
		}
	}

	pub fn optimize(&mut self) -> Vec<Bytecode> {
		let mut result = Vec::new();

		while let Some(cur) = Self::peek(self.bytes, self.index) {
			let inst = self.optimize_inst(&mut result);

			if inst {
				self.consume_next();
				self.consume_next();
			} else {
				result.push(cur.clone());

				self.consume_next();
			}
		}

		result
	}
}
