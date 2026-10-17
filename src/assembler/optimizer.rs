use crate::{assembler::Bytecode, compiler::native_func::NativeFunctionArgsNumber};

#[derive(Debug)]
pub struct AssemblerOptimizer<'a> {
	stack: Vec<(usize, &'a Bytecode)>,
	bytes: Option<&'a [Bytecode]>,
	index: usize,
}

impl<'a> AssemblerOptimizer<'a> {
	pub fn new(bytes: &'a [Bytecode]) -> Self {
		Self { stack: Vec::new(), bytes: Some(bytes), index: 0 }
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
				Bytecode::PushReg(_) |
				Bytecode::PushNumber(_) => {
					self.stack.push((result.len(), cur));

					self.next_inst(&mut result);
				}
				Bytecode::PopReg(reg) => {
					let Some((index, val)) = self.stack.pop() else {
						self.next_inst(&mut result);

						continue;
					};

					match *val {
						Bytecode::PushNumber(val) => {
							result.push(Bytecode::MovU32ToReg(reg, val));

							self.consumed_inst();
						},
						Bytecode::PushReg(reg1) => {
							result.push(Bytecode::MovRegToReg(reg, reg1));

							self.consumed_inst();
						}
						_ => {
							self.next_inst(&mut result);

							continue
						},
					}

					println!("-{}", result.remove(index));
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
							let Some((_, &Bytecode::PushNumber(fixed))) = self.stack.pop() else {
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

		result
	}
}
