use std::{collections::HashMap, fmt::Display};

use crate::parser::{ASTNode, ASTNodeEnum, BinaryOp, UnaryOp};

#[derive(PartialEq)]
pub struct Bytecode(u8);

impl Bytecode {
	pub const NOP: u8 = 0x00; pub const NOP_FF: u8 = 0xFF;
	pub const PUSH_NUMBER: u8 = 0x01;
	pub const PUSH_STRING: u8 = 0x02;
	pub const PUSH_FALSE: u8 = 0x03;
	pub const PUSH_TRUE: u8 = 0x04;
	pub const LOOK: u8 = 0x05; // Copy value from N
	pub const LOAD: u8 = 0x06; // Copy value to N
	pub const REMOVE: u8 = 0x07; // Remove value
	pub const PUSH_ARG_END: u8 = 0x0F;
	pub const JIT: u8 = 0x20; // Jump if condition
	pub const JIF: u8 = 0x21; // Jump if not condition
	pub const JMP: u8 = 0x28; // Jump without condition
	pub const EQ: u8 = 0x50;
	pub const NEQ: u8 = 0x51;
	pub const LT: u8 = 0x52; // Less than
	pub const GT: u8 = 0x53; // Great than
	pub const NATIVE_CALL: u8 = 0x80;
	pub const ADD: u8 = 0xA0;
	pub const SUB: u8 = 0xA3;
	pub const MUL: u8 = 0xB0;
	pub const DIV: u8 = 0xB3;
	pub const POW: u8 = 0xC0;
	pub const NEG: u8 = 0xC3;
	pub const NOT: u8 = 0xD0;
	pub const LOG_AND: u8 = 0xD3;
	pub const LOG_OR: u8 = 0xE0;
	// pub const POP: u8 = 0x10;

	pub fn from_unary_op(op: UnaryOp) -> u8 {
		match op {
			UnaryOp::Minus => Self::NEG,
			x => todo!("{}", x),
		}
	}

	pub fn from_binary_op(op: BinaryOp) -> u8 {
		match op {
			BinaryOp::Plus => Self::ADD,
			BinaryOp::Minus => Self::SUB,
			BinaryOp::Multiply => Self::MUL,
			BinaryOp::Divide => Self::DIV,
			BinaryOp::Pow => Self::POW,
			BinaryOp::Equals => Self::EQ,
			BinaryOp::NotEquals => Self::NEQ,
			BinaryOp::Less => Self::LT,
			BinaryOp::Great => Self::GT,
			BinaryOp::LogicalAnd => Self::LOG_AND,
			BinaryOp::LogicalOr => Self::LOG_OR,
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
	#[error("'{0}' variable is not known in current scope")]
	NotKnownAtThisScope(String),
	#[error("L-value of assignment should be variable name, not '{0}'")]
	LeftExprShouldBeId(String),
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
	stack_pos: u32,
	scope: HashMap<String, u32>,
}

impl Compiler {
	pub fn new() -> Self {
		Self {stack_pos: 0, scope: HashMap::new()}
	}

	pub fn get_native_call_id_by_name(name: &str) -> Option<NativeFunctionId> {
		match name {
			"print" => Some(NativeFunctionId::Print),
			"tuple" => Some(NativeFunctionId::Tuple),
			"range" => Some(NativeFunctionId::Range),
			_ => None,
		}
	}

	fn look(&mut self, stack_pos: u32, result: &mut Vec<u8>) {
		self.stack_pos += 1;

		result.push(Bytecode::LOOK);
		result.extend(stack_pos.to_le_bytes());
	}

	fn load(&mut self, stack_pos: u32, result: &mut Vec<u8>) {
		result.push(Bytecode::LOAD);
		result.extend(stack_pos.to_le_bytes());
	}

	fn push_bool(&mut self, val: bool, result: &mut Vec<u8>) {
		self.stack_pos += 1;

		if val {
			result.push(Bytecode::PUSH_TRUE);
		} else {
			result.push(Bytecode::PUSH_FALSE);
		}
	}

	fn push_number(&mut self, val: f64, result: &mut Vec<u8>) {
		self.stack_pos += 1;

		result.push(Bytecode::PUSH_NUMBER);
		result.extend(val.to_le_bytes());
	}

	fn push_string(&mut self, val: &str, result: &mut Vec<u8>) -> Result<(), CompilerError> {
		self.stack_pos += 1;

		let Ok(len_u16) = u16::try_from(val.len()) else {
			return Err(CompilerError::StringTooLong)
		};

		result.push(Bytecode::PUSH_STRING);
		result.extend(len_u16.to_le_bytes());
		result.extend(val.bytes());

		Ok(())
	}

	fn native_call_with_arg(&mut self, name: &str, arg: &ASTNode, result: &mut Vec<u8>) -> Result<(), CompilerError> {
		self.stack_pos += 1;

		result.push(Bytecode::PUSH_ARG_END);

		self.compile(arg, result)?;

		result.push(Bytecode::NATIVE_CALL);
		result.extend(Self::get_native_call_id_by_name(name).ok_or(
			CompilerError::UnknownNativeFunctionCall(name.to_owned())
		)?.to_le_bytes());

		Ok(())
	}

	fn native_call(&mut self, name: &str, args: &[ASTNode], result: &mut Vec<u8>) -> Result<(), CompilerError> {
		self.stack_pos += 1;

		result.push(Bytecode::PUSH_ARG_END);

		for arg in args.iter().rev() {
			self.compile(arg, result)?;
		}

		result.push(Bytecode::NATIVE_CALL);
		result.extend(Self::get_native_call_id_by_name(name).ok_or(
			CompilerError::UnknownNativeFunctionCall(name.to_owned())
		)?.to_le_bytes());

		Ok(())
	}

	fn jit(&mut self, pos: i32, result: &mut Vec<u8>) {
		self.stack_pos -= 1;

		result.push(Bytecode::JIT);
		result.extend(pos.to_le_bytes());
	}

	fn jif(&mut self, pos: i32, result: &mut Vec<u8>) {
		self.stack_pos -= 1;

		result.push(Bytecode::JIF);
		result.extend(pos.to_le_bytes());
	}

	fn jmp(pos: i32, result: &mut Vec<u8>) {
		result.push(Bytecode::JMP);
		result.extend(pos.to_le_bytes());
	}

	fn compile_assignment(&mut self, left: &ASTNode, right: &ASTNode, op: u8, result: &mut Vec<u8>) -> Result<(), CompilerError> {
		let ASTNodeEnum::Variable(name) = &left.value else {
			return Err(CompilerError::LeftExprShouldBeId(left.value.to_string()));
		};

		let Some(&stack_pos) = self.scope.get(name) else {
			return Err(CompilerError::NotKnownAtThisScope(name.clone()));
		};

		self.look(stack_pos, result);
		self.compile(right, result)?;

		result.push(op);

		self.load(stack_pos, result);

		Ok(())
	}

	pub fn compile(&mut self, ast: &ASTNode, result: &mut Vec<u8>) -> Result<(), CompilerError> {
		println!("stack_pos = {}", self.stack_pos);

		match &ast.value {
			ASTNodeEnum::None => return Ok(()),
			&ASTNodeEnum::Boolean(x) => {
				self.push_bool(x, result);
			},
			&ASTNodeEnum::Number(x) => {
				self.push_number(x, result);
			},
			ASTNodeEnum::String(x) => {
				self.push_string(x, result)?;
			},
			ASTNodeEnum::Tuple(values) => {
				self.native_call("tuple", values, result)?;
			},
			ASTNodeEnum::Binary { left, op: BinaryOp::Range, right } => {
				let args = vec![*left.clone(), *right.clone()];

				self.native_call("range", &args, result)?;
			},
			ASTNodeEnum::Binary { left, op: BinaryOp::Assignment, right } => if let ASTNodeEnum::Variable(name) = &left.value {
				if let Some(&stack_pos) = self.scope.get(name) {
					self.compile(right, result)?;

					self.load(stack_pos, result);
				} else {
					self.scope.insert(name.to_owned(), self.stack_pos);

					self.compile(right, result)?;
				}
			} else {
				return Err(CompilerError::LeftExprShouldBeId(left.value.to_string()));
			},
			ASTNodeEnum::Binary { left, op: BinaryOp::PlusAssignment, right } =>
				self.compile_assignment(left, right, Bytecode::ADD, result)?,
			ASTNodeEnum::Binary { left, op: BinaryOp::MinusAssignment, right } =>
				self.compile_assignment(left, right, Bytecode::SUB, result)?,
			ASTNodeEnum::Binary { left, op: BinaryOp::MultiplyAssignment, right } =>
				self.compile_assignment(left, right, Bytecode::MUL, result)?,
			ASTNodeEnum::Binary { left, op: BinaryOp::PowAssignment, right } =>
				self.compile_assignment(left, right, Bytecode::POW, result)?,
			ASTNodeEnum::Binary { left, op: BinaryOp::DivideAssignment, right } =>
				self.compile_assignment(left, right, Bytecode::DIV, result)?,
			ASTNodeEnum::Variable(name) => if let Some(&stack_pos) = self.scope.get(name) {
				self.look(stack_pos, result);
			} else {
				return Err(CompilerError::NotKnownAtThisScope(name.clone()));
			},
			ASTNodeEnum::Binary { left, op, right } => {
				self.compile(right, result)?;
				self.compile(left, result)?;

				self.stack_pos -= 1;
				self.stack_pos -= 1;

				result.push(Bytecode::from_binary_op(op.clone()));
			},
			ASTNodeEnum::Function { name, arg } => {
				self.native_call_with_arg(name, arg, result)?;
			},
			ASTNodeEnum::While { condition, block, block_else: None } => {
				let mut compiled_condition = Vec::new();
				self.compile(condition, &mut compiled_condition)?;

				let Ok(compiled_condition_size) = i32::try_from(compiled_condition.len()) else {
					return Err(CompilerError::BlockIsTooLong)
				};

				let mut compiled_block = Vec::new();
				self.compile(block, &mut compiled_block)?;

				let Ok(compiled_block_size) = i32::try_from(compiled_block.len()) else {
					return Err(CompilerError::BlockIsTooLong)
				};

				result.extend(compiled_condition);

				self.jif(compiled_block_size + 5, result);

				result.extend(compiled_block);
				Self::jmp(-compiled_block_size - compiled_condition_size - 10, result);
			},
			ASTNodeEnum::If { condition, block, block_else: Some(block_else) } => {
				let mut compiled_block = Vec::new();
				self.compile(block, &mut compiled_block)?;

				let Ok(else_offset) = i32::try_from(5 + compiled_block.len()) else {
					return Err(CompilerError::BlockIsTooLong)
				};

				let mut compiled_block_else = Vec::new();
				self.compile(block_else, &mut compiled_block_else)?;

				let Ok(else_block_size) = i32::try_from(compiled_block_else.len()) else {
					return Err(CompilerError::BlockIsTooLong)
				};

				self.compile(condition, result)?;

				self.stack_pos -= 1;

				self.jif(else_offset, result);

				result.extend(compiled_block);
				Self::jmp(else_block_size, result);

				result.extend(compiled_block_else);
			},
			ASTNodeEnum::If { condition, block, block_else: None } => {
				let mut compiled_block = Vec::new();

				self.compile(block, &mut compiled_block)?;

				let Ok(i32_len) = i32::try_from(compiled_block.len()) else {
					return Err(CompilerError::BlockIsTooLong)
				};

				self.compile(condition, result)?;

				self.jif(i32_len, result);

				result.extend(compiled_block);
			},
			ASTNodeEnum::Block(statements) => {
				let statements_len = statements.len();

				for (i, statement) in statements.iter().enumerate() {
					println!("statement = {statement}");

					self.compile(statement, result)?;
				}

				// result.push(Bytecode::REMOVE);
			},
			_ => todo!("{}", ast)
		}

		Ok(())
	}
}
