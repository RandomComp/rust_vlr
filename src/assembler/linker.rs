use crate::{assembler::{Bytecode, defs::BytecodeError}, compiler::Segments};

#[derive(thiserror::Error, Clone, Debug)]
pub enum LinkerError {
	#[error("{0}")]
	Bytecode(#[from] BytecodeError)
}

pub struct Linker();

impl Linker {
	pub fn link(segs: Segments) -> Result<Vec<u8>, LinkerError> {
		let mut result = Vec::new();

		result.extend(Bytecode::asm(segs.code)?);
		result.extend(segs.data);

		Ok(result)
	}
}
