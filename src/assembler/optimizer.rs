use crate::assembler::Bytecode;

#[derive(Debug)]
pub struct AssemblerOptimizer<'a> {
	bytes: Option<&'a [Bytecode]>,
}

impl<'a> AssemblerOptimizer<'a> {
	pub fn new(bytes: &'a [Bytecode]) -> Self {
		Self { bytes: Some(bytes) }
	}

	pub fn optimize(&mut self) {

	}
}
