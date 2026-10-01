use std::fmt::Display;

use crate::{lexer::TokenType, parser::{ASTNode, ASTNodeEnum}};

#[derive(PartialEq)]
pub struct Bytecode(u8);

impl Bytecode {
	pub const NOP: u8 = 0x00;
	pub const NOP_FF: u8 = 0xFF;
	pub const ADD: u8 = 0xA1;
	pub const SUB: u8 = 0xB1;
	pub const MUL: u8 = 0xC1;
	pub const DIV: u8 = 0xD1;
	pub const POW: u8 = 0xE1;
	pub const PUSH_NUMBER: u8 = 0x01;
	pub const PUSH_STRING: u8 = 0x02;
	pub const MAKE_TUPLE: u8 = 0x10;
	pub const PUSH_ARG_END: u8 = 0x0F;
	pub const NATIVE_CALL: u8 = 0x30;
	// pub const POP: u8 = 0x10;

	pub fn from_binary_op(op: TokenType) -> u8 {
		match op {
			TokenType::TokenNumber(_) => Self::PUSH_NUMBER,
			TokenType::TokenString(_) => Self::PUSH_STRING,
			TokenType::TokenPlus => Self::ADD,
			TokenType::TokenMinus => Self::SUB,
			TokenType::TokenMultiply => Self::MUL,
			TokenType::TokenDivide => Self::DIV,
			TokenType::TokenPow => Self::POW,
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
}

pub struct Compiler {
	// scope: Vec<String>,
}

impl Compiler {
	// pub fn new() -> Self {
	// 	Self {}
	// }

	pub fn get_native_call_id_by_name(name: &str) -> Option<u8> {
		match name {
			"print" => Some(0),
			_ => None,
		}
	}

	pub fn get_native_call_name_by_id(id: u8) -> Option<&'static str> {
		match id {
			0 => Some("print"),
			_ => None,
		}
	}

	pub fn compile(ast: &ASTNode, result: &mut Vec<u8>) -> Result<(), CompilerError> {
		match &ast.value {
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

				result.push(Bytecode::MAKE_TUPLE);
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
			_ => todo!("{}", ast)
		}

		Ok(())
	}
}
