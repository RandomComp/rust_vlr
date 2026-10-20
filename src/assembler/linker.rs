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

		let code_seg_size = 0x100;
		let data_seg_size = 0x100;

		let code_seg_offset = 0;
		let data_seg_offset = code_seg_size;

		result.extend(Bytecode::asm(segs.code)?);
		result.extend(segs.data);

		Ok(result)
	}
}
