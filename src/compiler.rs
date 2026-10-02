use std::fmt::Display;

use crate::{lexer::TokenType, parser::{ASTNode, ASTNodeEnum}};

#[derive(PartialEq)]
pub struct Bytecode(u8);

impl Bytecode {
	pub const NOP: u8 = 0x00; pub const NOP_FF: u8 = 0xFF;
	pub const PUSH_NUMBER: u8 = 0x01;
	pub const PUSH_STRING: u8 = 0x02;
	pub const PUSH_FALSE: u8 = 0x03;
	pub const PUSH_TRUE: u8 = 0x04;
	pub const COPY: u8 = 0x05; // Copies value from stack and pushes it up
	pub const SWAP: u8 = 0x06; // Swaps two values from stack
	pub const REMOVE: u8 = 0x07; // Remvoe value from stack
	pub const PUSH_ARG_END: u8 = 0x0F;
	pub const JIC: u8 = 0x20; // Jump if condition
	pub const JINC: u8 = 0x21; // Jump if not condition
	pub const JMP: u8 = 0x28; // Jump without condition
	pub const EQ: u8 = 0x50;
	pub const NEQ: u8 = 0x51;
	pub const NATIVE_CALL: u8 = 0x80;
	pub const ADD: u8 = 0xA0;
	pub const SUB: u8 = 0xA3;
	pub const MUL: u8 = 0xB0;
	pub const DIV: u8 = 0xB3;
	pub const POW: u8 = 0xC0;
	pub const NEG: u8 = 0xC3;
	pub const NOT: u8 = 0xD0;
	// pub const POP: u8 = 0x10;

	pub fn from_unary_op(op: TokenType) -> u8 {
		match op {
			TokenType::TokenMinus => Self::NEG,
			x => todo!("{}", x),
		}
	}

	pub fn from_binary_op(op: TokenType) -> u8 {
		match op {
			TokenType::TokenNumber(_) => Self::PUSH_NUMBER,
			TokenType::TokenString(_) => Self::PUSH_STRING,
			TokenType::TokenPlus => Self::ADD,
			TokenType::TokenMinus => Self::SUB,
			TokenType::TokenMultiply => Self::MUL,
			TokenType::TokenDivide => Self::DIV,
			TokenType::TokenPow => Self::POW,
			TokenType::TokenEquals => Self::EQ,
			TokenType::TokenNotEquals => Self::NEQ,
			x => todo!("{}", x),
		}
	}
}

impl Display for Bytecode {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    	write!(f, "{:2X}", self.0)
	}
}

#[derive(thiserror::Error, Debug)]
pub enum CompilerError {
	#[error("Unknown native function '{0}' call")]
	UnknownNativeFunctionCall(String),
	#[error("String constant length is too long for u16 type, that means string is bigger than 65 KiB, try using dynamic string")]
	StringTooLong,
	#[error("Block is too long for u32 type, that means block is bigger than 4 GB, try separate code for modules")]
	BlockIsTooLong,
}

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum NativeFunctionId {
	Print,
	Tuple,
	Range
}

impl Display for NativeFunctionId {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			NativeFunctionId::Print => write!(f, "print"),
			NativeFunctionId::Tuple => write!(f, "tuple"),
			NativeFunctionId::Range => write!(f, "range"),
		}
	}
}

impl NativeFunctionId {
	pub fn to_le_bytes(self) -> [u8; 1] {
		(self as u8).to_le_bytes()
	}

	pub fn get_name(self) -> &'static str {
		match self {
			NativeFunctionId::Print => "print",
			NativeFunctionId::Tuple => "tuple",
			NativeFunctionId::Range => "range",
		}
	}
}

impl From<u8> for NativeFunctionId {
	fn from(value: u8) -> Self {
		match value {
			0 => NativeFunctionId::Print,
			1 => NativeFunctionId::Tuple,
			2 => NativeFunctionId::Range,
			x => todo!("{x}")
		}
	}
}

pub struct Compiler {
	// scope: Vec<String>,
}

impl Compiler {
	// pub fn new() -> Self {
	// 	Self {}
	// }

	pub fn get_native_call_id_by_name(name: &str) -> Option<NativeFunctionId> {
		match name {
			"print" => Some(NativeFunctionId::Print),
			"tuple" => Some(NativeFunctionId::Tuple),
			"range" => Some(NativeFunctionId::Range),
			_ => None,
		}
	}

	pub fn compile(ast: &ASTNode, result: &mut Vec<u8>) -> Result<(), CompilerError> {
		match &ast.value {
			ASTNodeEnum::Boolean(x) if !*x => {
				result.push(Bytecode::PUSH_FALSE);
			},
			ASTNodeEnum::Boolean(x) if *x => {
				result.push(Bytecode::PUSH_TRUE);
			},
			ASTNodeEnum::Number(x) => {
				result.push(Bytecode::PUSH_NUMBER);
				result.extend(x.to_le_bytes());
			},
			ASTNodeEnum::String(x) => {
				let Ok(len_u16) = u16::try_from(x.len()) else {
					return Err(CompilerError::StringTooLong)
				};

				result.push(Bytecode::PUSH_STRING);
				result.extend(len_u16.to_le_bytes());
				result.extend(x.bytes());
			},
			ASTNodeEnum::Tuple(values) => {
				result.push(Bytecode::PUSH_ARG_END);

				for value in values.iter().rev() {
					Self::compile(value, result)?;
				}

				result.push(Bytecode::NATIVE_CALL);
				result.extend(Self::get_native_call_id_by_name("tuple").ok_or(
					CompilerError::UnknownNativeFunctionCall("tuple".to_owned())
				)?.to_le_bytes());
			},
			ASTNodeEnum::Binary { left, op: TokenType::TokenRange, right } => {
				result.push(Bytecode::PUSH_ARG_END);

				Self::compile(right, result)?;
				Self::compile(left, result)?;

				result.push(Bytecode::NATIVE_CALL);
				result.extend(Self::get_native_call_id_by_name("range").ok_or(
					CompilerError::UnknownNativeFunctionCall("range".to_owned())
				)?.to_le_bytes());
			},
			ASTNodeEnum::Binary { left, op, right } => {
				Self::compile(left, result)?;
				Self::compile(right, result)?;

				result.push(Bytecode::from_binary_op(op.clone()));
			},
			ASTNodeEnum::Function { name, arg } => {
				result.push(Bytecode::PUSH_ARG_END);

				Self::compile(arg, result)?;

				result.push(Bytecode::NATIVE_CALL);

				// let id = if self.scope.contains(&name) {
				// 	self.scope.iter().position(|v| *v == name).unwrap()
				// } else {
				// 	self.scope.push(name);

				// 	self.scope.len() - 1
				// };

				// println!("id = {id}");

				result.extend(Self::get_native_call_id_by_name(name).ok_or(
					CompilerError::UnknownNativeFunctionCall(name.clone())
				)?.to_le_bytes());
			},
			ASTNodeEnum::While { condition, block, block_else: None } => {
				let mut compiled_block = Vec::new();
				Self::compile(block, &mut compiled_block)?;

				let Ok(compiled_block_size) = i32::try_from(compiled_block.len()) else {
					return Err(CompilerError::BlockIsTooLong)
				};

				Self::compile(condition, result)?;

				result.push(Bytecode::JINC);
				result.extend((compiled_block_size + 5).to_le_bytes());

				result.extend(compiled_block);
				result.push(Bytecode::JMP);
				result.extend((-compiled_block_size).to_le_bytes());
			},
			ASTNodeEnum::If { condition, block, block_else: Some(block_else) } => {
				let mut compiled_block = Vec::new();
				Self::compile(block, &mut compiled_block)?;

				let Ok(else_offset) = i32::try_from(5 + compiled_block.len()) else {
					return Err(CompilerError::BlockIsTooLong)
				};

				let mut compiled_block_else = Vec::new();
				Self::compile(block_else, &mut compiled_block_else)?;

				let Ok(else_block_size) = i32::try_from(compiled_block_else.len()) else {
					return Err(CompilerError::BlockIsTooLong)
				};

				Self::compile(condition, result)?;

				result.push(Bytecode::JINC);
				result.extend(else_offset.to_le_bytes());

				result.extend(compiled_block);
				result.push(Bytecode::JMP);
				result.extend(else_block_size.to_le_bytes());

				result.extend(compiled_block_else);
			},
			ASTNodeEnum::If { condition, block, block_else: None } => {
				let mut compiled_block = Vec::new();

				Self::compile(block, &mut compiled_block)?;

				let Ok(i32_len) = i32::try_from(compiled_block.len()) else {
					return Err(CompilerError::BlockIsTooLong)
				};

				Self::compile(condition, result)?;
				result.push(Bytecode::JINC);
				result.extend(i32_len.to_le_bytes());

				result.extend(compiled_block);
			},
			ASTNodeEnum::Block(statements) => {
				for statement in statements {
					Self::compile(statement, result)?;
				}
			},
			_ => todo!("{}", ast)
		}

		Ok(())
	}
}
