use std::fmt::Display;

use crate::{compiler::NativeFunctionId, parser::{BinaryOp, UnaryOp}};

#[derive(PartialEq, Clone, Debug)]
pub enum Bytecode {
	/// Do nothing
	Halt,
	/// Pushes number
	PushFloat(f64),
	/// Pushes string to stack
	PushString(String),
	/// Pushes boolean to stack
	PushBoolean(bool),
	/// Copy value from stack N
	Look(u32),
	/// Copy value to stack at N
	Load(u32),
	/// Remove value from stack upper
	Remove,
	/// Sentinel for native functions arguments
	PushArgEnd,
	/// Jump if condition
	Jit(u32),
	/// Jump if not condition
	Jif(u32),
	/// Jump without condition
	Jmp(u32),
	/// Jump to pc from stack
	Call(u32),
	/// Starts subprogram (pushes stack value to interal ret stack)
	Subprogram,
	/// Returns from subprogram
	Ret,
	/// Equals
	Eq,
	/// Not equals
	Neq,
	/// Less than
	Lt,
	/// Great than
	Gt,
	NativeCall(NativeFunctionId),
	/// Additive
	Add,
	/// Subtraction
	Sub,
	/// Multiplication
	Mul,
	/// Division
	Div,
	/// Power of
	Pow,
	/// Negative value (-)
	Neg,
	/// Logical not
	Not,
	/// Logical AND
	LogicalAnd,
	/// Logical OR
	LogicalOr,
}

fn consume_const_bytes_and_get<T, const LEN: usize>(arr: &mut T) -> Option<[u8; LEN]> where T: Iterator<Item=u8> {
	let mut result = [0u8; LEN];

	for val in &mut result {
		*val = arr.next()?;
	}

	Some(result)
}

impl Display for Bytecode {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Bytecode::Halt =>
				write!(f, "hlt"),
			Bytecode::PushBoolean(v) =>
				write!(f, "push {v}"),
			Bytecode::PushFloat(val) =>
				write!(f, "push {val}"),
			Bytecode::PushString(text) =>
				write!(f, "push \"{text}\""),
			Bytecode::Look(stack_pos) =>
				write!(f, "look {stack_pos}"),
			Bytecode::Load(stack_pos) =>
				write!(f, "load {stack_pos}"),
			Bytecode::Remove =>
				write!(f, "remove"),
			Bytecode::PushArgEnd =>
				write!(f, "push args_end"),
			Bytecode::Add =>
				write!(f, "add"),
			Bytecode::Sub =>
				write!(f, "sub"),
			Bytecode::Mul =>
				write!(f, "mul"),
			Bytecode::Div =>
				write!(f, "div"),
			Bytecode::Pow =>
				write!(f, "pow"),
			Bytecode::Eq =>
				write!(f, "eq"),
			Bytecode::Neq =>
				write!(f, "neq"),
			Bytecode::Lt =>
				write!(f, "lt"),
			Bytecode::Gt =>
				write!(f, "gt"),
			Bytecode::LogicalAnd =>
				write!(f, "log_and"),
			Bytecode::LogicalOr =>
				write!(f, "log_or"),
			Bytecode::Neg =>
				write!(f, "neg"),
			Bytecode::Not =>
				write!(f, "not"),
			Bytecode::Jit(pos) =>
				write!(f, "jit {pos:02}"),
			Bytecode::Jif(pos) =>
				write!(f, "jif {pos:02}"),
			Bytecode::Jmp(pos) =>
				write!(f, "jmp {pos:02}"),
			Bytecode::Call(pos) =>
				write!(f, "call {pos:02}"),
			Bytecode::Subprogram =>
				write!(f, "subprogram"),
			Bytecode::Ret =>
				write!(f, "ret"),
			Bytecode::NativeCall(id) =>
				write!(f, "native_call {}", id.get_name())
		}
	}
}

#[derive(thiserror::Error, Debug)]
pub enum BytecodeError {
	#[error("Byte {byte:02X} at {pos:02} unexpected")]
	UnexpectedByte {
		pos: u32, byte: u8
	},
	#[error("Unexpected EOF at {pos}")]
	UnexpectedEof {
		pos: u32,
	},
}

impl Bytecode {
	pub const HLT: u8 = 0x00; pub const HLT_FF: u8 = 0xFF; // Halt VM
	pub const PUSH_FLOAT: u8 = 0x01; // Pushes number
	pub const PUSH_STRING: u8 = 0x02; // Pushes string to stack (needing len)
	pub const PUSH_FALSE: u8 = 0x03; // Pushes false to stack
	pub const PUSH_TRUE: u8 = 0x04; // Pushes true to stack
	pub const LOOK: u8 = 0x05; // Copy value from N
	pub const LOAD: u8 = 0x06; // Copy value to N
	pub const REMOVE: u8 = 0x07; // Remove value
	pub const PUSH_ARG_END: u8 = 0x0F;
	pub const JIT: u8 = 0x20; // Jump if condition
	pub const JIF: u8 = 0x21; // Jump if not condition
	pub const JMP: u8 = 0x28; // Jump without condition
	pub const CALL: u8 = 0x2A; // Jump to pc from stack
	pub const SUBPROGRAM: u8 = 0x2B;
	pub const RET: u8 = 0x2C; // Jump to pc from stack
	pub const EQ: u8 = 0x50; // Equals
	pub const NEQ: u8 = 0x51; // Not equals
	pub const LT: u8 =  0x52; // Less than
	pub const GT: u8 =  0x53; // Great than
	pub const NATIVE_CALL: u8 =  0x80;
	pub const ADD: u8 =  0xA0; // Additive
	pub const SUB: u8 =  0xA3; // Subtraction
	pub const MUL: u8 =  0xB0; // Multiplication
	pub const DIV: u8 =  0xB3; // Division
	pub const POW: u8 =  0xC0; // Power of
	pub const NEG: u8 =  0xC3; // Negative value (-)
	pub const NOT: u8 =  0xD0; // Logical not
	pub const LOGICAL_AND: u8 =  0xD3; // Logical AND
	pub const LOGICAL_OR: u8 =  0xE0; // Logical OR

	pub fn to_binary_op(&self) -> Option<BinaryOp> {
		let result = match self {
			Bytecode::Eq 					=> BinaryOp::Equals, // Equals
			Bytecode::Neq 					=> BinaryOp::NotEquals, // Not equals
			Bytecode::Lt 					=> BinaryOp::Less, // Less than
			Bytecode::Gt 					=> BinaryOp::Great, // Great than
			Bytecode::Add 					=> BinaryOp::Plus, // Additive
			Bytecode::Sub 					=> BinaryOp::Minus, // Subtraction
			Bytecode::Mul 					=> BinaryOp::Multiply, // Multiplication
			Bytecode::Div 					=> BinaryOp::Divide, // Division
			Bytecode::Pow 					=> BinaryOp::Pow, // Power of
			Bytecode::LogicalAnd 			=> BinaryOp::LogicalAnd, // Logical AND
			Bytecode::LogicalOr 			=> BinaryOp::LogicalOr, // Logical OR
			_ => return None,
		};

		Some(result)
	}

	pub fn to_unary_op(&self) -> Option<UnaryOp> {
		let result = match self {
			Self::Neg => UnaryOp::Minus, // Negative value (-)
			Self::Not => UnaryOp::Not, // Logical not
			_ => return None,
		};

		Some(result)
	}

	fn get_opcode(&self) -> u8 {
		match self {
			Self::Halt 					=> Self::HLT, // Halt VM
			Self::PushFloat(..) 		=> Self::PUSH_FLOAT, // Pushes number
			Self::PushString(..) 		=> Self::PUSH_STRING, // Pushes string to stack
			Self::PushBoolean(false) 	=> Self::PUSH_FALSE, // Pushes false to stack
			Self::PushBoolean(true) 	=> Self::PUSH_TRUE, // Pushes true to stack
			Self::Look(..) 				=> Self::LOOK, // Copy value from N
			Self::Load(..) 				=> Self::LOAD, // Copy value to N
			Self::Remove 				=> Self::REMOVE, // Remove value
			Self::PushArgEnd 			=> Self::PUSH_ARG_END,
			Self::Jit(..) 				=> Self::JIT, // Jump if condition
			Self::Jif(..) 				=> Self::JIF, // Jump if not condition
			Self::Jmp(..) 				=> Self::JMP, // Jump without condition
			Self::Call(..) 				=> Self::CALL, // Jump to pc from stack
			Self::Subprogram 			=> Self::SUBPROGRAM, // Jump to pc from stack
			Self::Ret 					=> Self::RET, // Jump to pc from stack
			Self::Eq 					=> Self::EQ, // Equals
			Self::Neq 					=> Self::NEQ, // Not equals
			Self::Lt 					=> Self::LT, // Less than
			Self::Gt 					=> Self::GT, // Great than
			Self::NativeCall(..) 		=> Self::NATIVE_CALL,
			Self::Add 					=> Self::ADD, // Additive
			Self::Sub 					=> Self::SUB, // Subtraction
			Self::Mul 					=> Self::MUL, // Multiplication
			Self::Div 					=> Self::DIV, // Division
			Self::Pow 					=> Self::POW, // Power of
			Self::Neg 					=> Self::NEG, // Negative value (-)
			Self::Not 					=> Self::NOT, // Logical not
			Self::LogicalAnd 			=> Self::LOGICAL_AND, // Logical AND
			Self::LogicalOr 			=> Self::LOGICAL_OR, // Logical OR
		}
	}

	fn asm_inst(value: Self) -> Vec<u8> {
		let mut result = Vec::new();

		result.push(value.get_opcode());

		match value {
			Self::PushFloat(x) => {
				result.extend(x.to_le_bytes());
			}, // Pushes number
			Self::PushString(text) => {
				let u32_len = u32::try_from(text.len()).unwrap();

				result.extend(u32_len.to_le_bytes());
				result.extend(text.bytes());
			}, // Pushes string to stack
			Self::Look(pos) |
			Self::Load(pos) |
			Self::Jit(pos) |
			Self::Jif(pos) |
			Self::Jmp(pos) |
			Self::Call(pos) => {
				result.extend(pos.to_le_bytes());
			}, // Jump to pc from stack
			Self::NativeCall(id) => {
				result.extend(id.to_le_bytes());
			},
			_ => {},
		}

		result
	}

	pub fn asm(values: Vec<Self>) -> Vec<u8> {
		let mut result = Vec::new();

		for value in values {
			let res: Vec<u8> = Self::asm_inst(value);

			result.extend(res);
		}

		result
	}

	pub fn disasm_inst<T>(mut bytes: T) -> Result<Option<Self>, BytecodeError> where T: Iterator<Item = u8> {
		let Some(opcode) = bytes.next() else {
			return Ok(None)
		};

		let result = match opcode {
			Self::HLT | Self::HLT_FF => Self::Halt, // Do nothing
			Self::PUSH_FLOAT => { // Pushes number
				let value = f64::from_le_bytes(consume_const_bytes_and_get::<T, 8>(&mut bytes).unwrap());

				Self::PushFloat(value)
			},
			Self::PUSH_STRING => { // Pushes string to stack (needing len)
				let u32_len = u32::from_le_bytes(consume_const_bytes_and_get::<T, 4>(&mut bytes).unwrap());

				let utf8 = bytes.take(u32_len as usize).collect();

				let text = String::from_utf8(utf8).unwrap();

				Self::PushString(text)
			},
			Self::PUSH_FALSE => Self::PushBoolean(false), // Pushes false to stack
			Self::PUSH_TRUE => Self::PushBoolean(true), // Pushes true to stack
			Self::LOOK => { // Copy value from N
				let stack_pos = u32::from_le_bytes(consume_const_bytes_and_get::<T, 4>(&mut bytes).unwrap());

				Self::Look(stack_pos)
			},
			Self::LOAD => { // Copy value to N
				let stack_pos = u32::from_le_bytes(consume_const_bytes_and_get::<T, 4>(&mut bytes).unwrap());

				Self::Load(stack_pos)
			},
			Self::REMOVE => Self::Remove, // Remove value
			Self::PUSH_ARG_END => Self::PushArgEnd,
			Self::JIT => { // Jump if condition
				let pos = u32::from_le_bytes(consume_const_bytes_and_get::<T, 4>(&mut bytes).unwrap());

				Self::Jit(pos)
			},
			Self::JIF => { // Jump if not condition
				let pos = u32::from_le_bytes(consume_const_bytes_and_get::<T, 4>(&mut bytes).unwrap());

				Self::Jif(pos)
			},
			Self::JMP => { // Jump without condition
				let pos = u32::from_le_bytes(consume_const_bytes_and_get::<T, 4>(&mut bytes).unwrap());

				Self::Jmp(pos)
			},
			Self::CALL => { // Call subprogram (like jmp, but before pushes pc)
				let pos = u32::from_le_bytes(consume_const_bytes_and_get::<T, 4>(&mut bytes).unwrap());

				Self::Call(pos)
			},
			Self::SUBPROGRAM => Self::Subprogram,
			Self::RET => Self::Ret, // Jump to pc from stack
			Self::EQ => Self::Eq, // Equals
			Self::NEQ => Self::Neq, // Not equals
			Self::LT => Self::Lt, // Less than
			Self::GT => Self::Gt, // Great than
			Self::NATIVE_CALL => {
				let id = NativeFunctionId::try_from(bytes.nth(0).unwrap()).unwrap();

				Self::NativeCall(id)
			},
			Self::ADD 			=> Self::Add, // Additive
			Self::SUB 			=> Self::Sub, // Subtraction
			Self::MUL 			=> Self::Mul, // Multiplication
			Self::DIV 			=> Self::Div, // Division
			Self::POW 			=> Self::Pow, // Power of
			Self::NEG 			=> Self::Neg, // Negative value (-)
			Self::NOT 			=> Self::Not, // Logical not
			Self::LOGICAL_AND 	=> Self::LogicalAnd, // Logical AND
			Self::LOGICAL_OR 	=> Self::LogicalOr, // Logical OR
			x => return Err(BytecodeError::UnexpectedByte { pos: 0, byte: x }),
		};

		Ok(Some(result))
	}

	pub fn disasm<T>(mut bytes: T) -> Result<Vec<Self>, BytecodeError> where T: Iterator<Item = u8> {
		let mut result = Vec::new();

		while let Some(val) = Self::disasm_inst(&mut bytes)? {
			result.push(val);
		}

		Ok(result)
	}

	pub fn from_unary_op(op: &UnaryOp) -> Bytecode {
		match op {
			UnaryOp::Minus => Self::Neg,
			UnaryOp::Not => Self::Not,
		}
	}

	pub fn from_binary_op(op: &BinaryOp) -> Bytecode {
		match op {
			BinaryOp::Plus => Self::Add,
			BinaryOp::Minus => Self::Sub,
			BinaryOp::Multiply => Self::Mul,
			BinaryOp::Divide => Self::Div,
			BinaryOp::Pow => Self::Pow,
			BinaryOp::Equals => Self::Eq,
			BinaryOp::NotEquals => Self::Neq,
			BinaryOp::Less => Self::Lt,
			BinaryOp::Great => Self::Gt,
			BinaryOp::LogicalAnd => Self::LogicalAnd,
			BinaryOp::LogicalOr => Self::LogicalOr,
			x => todo!("{}", x),
		}
	}
}
