use std::{collections::HashMap, fmt::Display, iter};

use crate::parser::{ASTNode, ASTNodeEnum, BinaryOp, UnaryOp};

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
	/// Jump to pc from stack
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
	pub const RET: u8 = 0x2B; // Jump to pc from stack
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

	fn get_opcode(&self) -> u8 {
		match self {
			Bytecode::Halt 					=> Self::HLT, // Halt VM
			Bytecode::PushFloat(..) 		=> Self::PUSH_FLOAT, // Pushes number
			Bytecode::PushString(..) 		=> Self::PUSH_STRING, // Pushes string to stack
			Bytecode::PushBoolean(false) 	=> Self::PUSH_FALSE, // Pushes false to stack
			Bytecode::PushBoolean(true) 	=> Self::PUSH_TRUE, // Pushes true to stack
			Bytecode::Look(..) 				=> Self::LOOK, // Copy value from N
			Bytecode::Load(..) 				=> Self::LOAD, // Copy value to N
			Bytecode::Remove 				=> Self::REMOVE, // Remove value
			Bytecode::PushArgEnd 			=> Self::PUSH_ARG_END,
			Bytecode::Jit(..) 				=> Self::JIT, // Jump if condition
			Bytecode::Jif(..) 				=> Self::JIF, // Jump if not condition
			Bytecode::Jmp(..) 				=> Self::JMP, // Jump without condition
			Bytecode::Call(..) 				=> Self::CALL, // Jump to pc from stack
			Bytecode::Ret 					=> Self::RET, // Jump to pc from stack
			Bytecode::Eq 					=> Self::EQ, // Equals
			Bytecode::Neq 					=> Self::NEQ, // Not equals
			Bytecode::Lt 					=> Self::LT, // Less than
			Bytecode::Gt 					=> Self::GT, // Great than
			Bytecode::NativeCall(..) 		=> Self::NATIVE_CALL,
			Bytecode::Add 					=> Self::ADD, // Additive
			Bytecode::Sub 					=> Self::SUB, // Subtraction
			Bytecode::Mul 					=> Self::MUL, // Multiplication
			Bytecode::Div 					=> Self::DIV, // Division
			Bytecode::Pow 					=> Self::POW, // Power of
			Bytecode::Neg 					=> Self::NEG, // Negative value (-)
			Bytecode::Not 					=> Self::NOT, // Logical not
			Bytecode::LogicalAnd 			=> Self::LOGICAL_AND, // Logical AND
			Bytecode::LogicalOr 			=> Self::LOGICAL_OR, // Logical OR
		}
	}

	fn asm_inst(value: Bytecode) -> Result<Vec<u8>, BytecodeError> {
		let mut result = Vec::new();

		result.push(value.get_opcode());

		match value {
			Bytecode::PushFloat(x) => {
				result.extend(x.to_le_bytes());
			}, // Pushes number
			Bytecode::PushString(text) => {
				let u32_len = u32::try_from(text.len()).unwrap();

				result.extend(u32_len.to_le_bytes());
				result.extend(text.bytes());
			}, // Pushes string to stack
			Bytecode::Look(stack_pos) => {
				result.extend(stack_pos.to_le_bytes());
			}, // Copy value from N
			Bytecode::Load(stack_pos) => {
				result.extend(stack_pos.to_le_bytes());
			}, // Copy value to N
			Bytecode::Jit(pos) => {
				result.extend(pos.to_le_bytes());
			}, // Jump if condition
			Bytecode::Jif(pos) => {
				result.extend(pos.to_le_bytes());
			}, // Jump if not condition
			Bytecode::Jmp(pos) => {
				result.extend(pos.to_le_bytes());
			}, // Jump without condition
			Bytecode::Call(pos) => {
				result.extend(pos.to_le_bytes());
			}, // Jump to pc from stack
			Bytecode::NativeCall(id) => {
				result.extend(id.to_le_bytes());
			},
			_ => {},
		}

		Ok(result)
	}

	pub fn asm(values: Vec<Self>) -> Result<Vec<u8>, BytecodeError> {
		let mut result = Vec::new();

		for value in values {
			let res: Vec<u8> = Self::asm_inst(value)?;

			result.extend(res);
		}

		Ok(result)
	}

	pub fn disasm_inst<T>(mut bytes: T) -> Result<Option<Self>, BytecodeError> where T: Iterator<Item = u8> {
		let Some(opcode) = bytes.next() else {
			return Ok(None)
		};

		let result = match opcode {
			Bytecode::HLT | Bytecode::HLT_FF => Self::Halt, // Do nothing
			Bytecode::PUSH_FLOAT => { // Pushes number
				let value = f64::from_le_bytes(consume_const_bytes_and_get::<T, 8>(&mut bytes).unwrap());

				Self::PushFloat(value)
			},
			Bytecode::PUSH_STRING => { // Pushes string to stack (needing len)
				let u32_len = u32::from_le_bytes(consume_const_bytes_and_get::<T, 4>(&mut bytes).unwrap());

				let utf8 = bytes.take(u32_len as usize).collect();

				let text = String::from_utf8(utf8).unwrap();

				Self::PushString(text)
			},
			0x03 => Self::PushBoolean(false), // Pushes false to stack
			0x04 => Self::PushBoolean(true), // Pushes true to stack
			0x05 => { // Copy value from N
				let stack_pos = u32::from_le_bytes(consume_const_bytes_and_get::<T, 4>(&mut bytes).unwrap());

				Self::Look(stack_pos)
			},
			0x06 => { // Copy value to N
				let stack_pos = u32::from_le_bytes(consume_const_bytes_and_get::<T, 4>(&mut bytes).unwrap());

				Self::Load(stack_pos)
			},
			0x07 => Self::Remove, // Remove value
			0x0F => Self::PushArgEnd,
			0x20 => { // Jump if condition
				let pos = u32::from_le_bytes(consume_const_bytes_and_get::<T, 4>(&mut bytes).unwrap());

				Self::Jit(pos)
			},
			0x21 => { // Jump if not condition
				let pos = u32::from_le_bytes(consume_const_bytes_and_get::<T, 4>(&mut bytes).unwrap());

				Self::Jif(pos)
			},
			0x28 => { // Jump without condition
				let pos = u32::from_le_bytes(consume_const_bytes_and_get::<T, 4>(&mut bytes).unwrap());

				Self::Jmp(pos)
			},
			0x2A => { // Call subprogram (like jmp, but before pushes pc)
				let pos = u32::from_le_bytes(consume_const_bytes_and_get::<T, 4>(&mut bytes).unwrap());

				Self::Call(pos)
			},
			0x2B => Self::Ret, // Jump to pc from stack
			0x50 => Self::Eq, // Equals
			0x51 => Self::Neq, // Not equals
			0x52 => Self::Lt, // Less than
			0x53 => Self::Gt, // Great than
			0x80 => {
				let id = NativeFunctionId::try_from(bytes.nth(0).unwrap()).unwrap();

				Self::NativeCall(id)
			},
			0xA0 => Self::Add, // Additive
			0xA3 => Self::Sub, // Subtraction
			0xB0 => Self::Mul, // Multiplication
			0xB3 => Self::Div, // Division
			0xC0 => Self::Pow, // Power of
			0xC3 => Self::Neg, // Negative value (-)
			0xD0 => Self::Not, // Logical not
			0xD3 => Self::LogicalAnd, // Logical AND
			0xE0 => Self::LogicalOr, // Logical OR
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

// impl Display for Bytecode {
// 	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
// 		let value: u8 = self.into();

// 		write!(f, "{value:2X}")
// 	}
// }

#[derive(thiserror::Error, Debug)]
pub enum CompilerError {
	#[error("Unknown function '{0}' call")]
	UnknownFunctionCall(String),
	#[error("String constant length is too long for u16 type, that means string is bigger than 65 KiB, try using dynamic string")]
	StringTooLong,
	#[error("Block is too long for u32 type, that means block is bigger than 4 GB, try separate code for modules")]
	BlockIsTooLong,
	#[error("'{0}' variable is not known in current scope")]
	NotKnownAtThisScope(String),
	#[error("L-value of assignment should be variable name, not '{0}'")]
	LeftExprShouldBeId(String),
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
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

impl TryFrom<u8> for NativeFunctionId {
	type Error = ();

	fn try_from(value: u8) -> Result<Self, Self::Error> {
		match value {
			0 => Ok(NativeFunctionId::Print),
			1 => Ok(NativeFunctionId::Tuple),
			2 => Ok(NativeFunctionId::Range),
			_ => Err(()),
		}
	}
}

impl TryFrom<&str> for NativeFunctionId {
	type Error = ();

	fn try_from(name: &str) -> Result<Self, Self::Error> {
		match name {
			"print" => Ok(NativeFunctionId::Print),
			"tuple" => Ok(NativeFunctionId::Tuple),
			"range" => Ok(NativeFunctionId::Range),
			_ => Err(()),
		}
	}
}

struct CompilerContext<'a> {
	functions: HashMap<String, u32>,
	scope: HashMap<String, u32>,
	parent: Option<&'a CompilerContext<'a>>,
}

impl<'a> CompilerContext<'a> {
	fn new(parent: Option<&'a CompilerContext>) -> Self {
		Self {scope: HashMap::new(), functions: HashMap::new(), parent}
	}

	fn get_var(&self, name: &str) -> Option<u32> {
		if let Some(value) = self.scope.get(name) {
			Some(*value)
		} else if let Some(parent) = self.parent {
			parent.scope.get(name).copied()
		} else {
			None
		}
	}
	fn get_function(&self, name: &str) -> Option<u32> {
		if let Some(value) = self.functions.get(name) {
			Some(*value)
		} else if let Some(parent) = self.parent {
			parent.functions.get(name).copied()
		} else {
			None
		}
	}

	fn insert_var<T>(&mut self, name: T, stack_pos: u32) where T: Into<String> {
		self.scope.insert(name.into(), stack_pos);
	}
	fn insert_function<T>(&mut self, name: T, pos: u32) where T: Into<String> {
		self.functions.insert(name.into(), pos);
	}
}

pub struct Compiler {
	stack_pos: u32,
	pc: u32,
}

impl Compiler {
	pub fn new() -> Self {
		Self {stack_pos: 0, pc: 0}
	}

	fn look(&mut self, stack_pos: u32, result: &mut Vec<Bytecode>) {
		self.stack_pos += 1;

		result.push(Bytecode::Look(stack_pos));

		self.pc += 1 + 4;
	}

	fn load(&mut self, stack_pos: u32, result: &mut Vec<Bytecode>) {
		result.push(Bytecode::Load(stack_pos));

		self.pc += 1 + 4;
	}

	fn push_bool(&mut self, val: bool, result: &mut Vec<Bytecode>) {
		self.stack_pos += 1;

		result.push(Bytecode::PushBoolean(val));

		self.pc += 1;
	}

	fn push_float(&mut self, val: f64, result: &mut Vec<Bytecode>) {
		self.stack_pos += 1;

		result.push(Bytecode::PushFloat(val));

		self.pc += 1 + 8;
	}

	fn push_string(&mut self, val: String, result: &mut Vec<Bytecode>) -> Result<(), CompilerError> {
		self.stack_pos += 1;

		let Ok(len_u32) = u32::try_from(val.len()) else {
			return Err(CompilerError::StringTooLong)
		};

		result.push(Bytecode::PushString(val));

		self.pc += 1 + 2 + len_u32;

		Ok(())
	}

	fn push_arg_end(&mut self, result: &mut Vec<Bytecode>) {
		self.stack_pos += 1;

		result.push(Bytecode::PushArgEnd);

		self.pc += 1;
	}

	fn call_func<'a, T>(&mut self, name: &str, args: T, result: &mut Vec<Bytecode>, context: &mut CompilerContext) -> Result<(), CompilerError> where T: DoubleEndedIterator<Item=&'a ASTNode> {
		if let Ok(id) = NativeFunctionId::try_from(name) {
			self.push_arg_end(result);

			let mut len = 1;

			for arg in args.rev() {
				self.compile(arg, result, context)?;

				len += 1;
			}

			self.stack_pos -= len;

			result.push(Bytecode::NativeCall(id));

			self.pc += 1 + 1;
		} else if let Some(pos) = context.get_function(name) {
			self.call(pos, result);
		} else {
			return Err(CompilerError::UnknownFunctionCall(name.to_owned()))
		}

		Ok(())
	}

	fn jit(&mut self, pos: u32, result: &mut Vec<Bytecode>) {
		self.stack_pos -= 1;

		result.push(Bytecode::Jit(pos));

		self.pc += 1 + 4;
	}

	fn jif(&mut self, pos: u32, result: &mut Vec<Bytecode>) {
		self.stack_pos -= 1;

		result.push(Bytecode::Jif(pos));

		self.pc += 1 + 4;
	}

	fn jmp(&mut self, pos: u32, result: &mut Vec<Bytecode>) {
		result.push(Bytecode::Jmp(pos));

		self.pc += 1 + 4;
	}

	fn call(&mut self, pos: u32, result: &mut Vec<Bytecode>) {
		result.push(Bytecode::Call(pos));

		self.pc += 1 + 4;
	}

	fn ret(&mut self, result: &mut Vec<Bytecode>) {
		result.push(Bytecode::Ret);

		self.pc += 1;
	}

	fn compile_assignment(&mut self, left: &ASTNode, right: &ASTNode, op: Bytecode, result: &mut Vec<Bytecode>, context: &mut CompilerContext) -> Result<(), CompilerError> {
		let ASTNodeEnum::Variable(name) = &left.value else {
			return Err(CompilerError::LeftExprShouldBeId(left.value.to_string()));
		};

		let Some(stack_pos) = context.get_var(name) else {
			return Err(CompilerError::NotKnownAtThisScope(name.to_owned()));
		};

		self.compile(right, result, context)?;
		self.look(stack_pos, result);

		result.push(op);
		self.pc += 1;

		self.load(stack_pos, result);

		Ok(())
	}

	fn compile(&mut self, ast: &ASTNode, result: &mut Vec<Bytecode>, context: &mut CompilerContext) -> Result<(), CompilerError> {
		println!("stack_pos = {}", self.stack_pos);

		match &ast.value {
			ASTNodeEnum::FunctionDefinition { name: Some(name), args, block } => {
				let mut compiled_block = Vec::new();

				self.compile(block, &mut compiled_block, context)?;

				let Ok(compiled_block_len) = u32::try_from(compiled_block.len()) else {
					return Err(CompilerError::BlockIsTooLong)
				};

				self.jmp(self.pc + compiled_block_len + 6, result);

				context.insert_function(name.to_owned(), self.pc);

				result.extend(compiled_block);

				self.ret(result);
			},
			ASTNodeEnum::While { block_else: Some(_), .. } |
			ASTNodeEnum::Break(..) | ASTNodeEnum::Return(..) |
			ASTNodeEnum::FunctionDefinition {..} => todo!("{}", ast.value),
			ASTNodeEnum::None => return Ok(()),
			&ASTNodeEnum::Boolean(x) => {
				self.push_bool(x, result);
			},
			&ASTNodeEnum::Float(x) => {
				self.push_float(x, result);
			},
			ASTNodeEnum::String(x) => {
				self.push_string(x.clone(), result)?;
			},
			ASTNodeEnum::Tuple(values) => {
				self.call_func("tuple", values.iter(), result, context)?;
			},
			ASTNodeEnum::Binary { left, op: BinaryOp::Range, right } => {
				self.call_func("range", [&**left, &**right].into_iter(), result, context)?;
			},
			ASTNodeEnum::Binary { left, op: BinaryOp::Assignment, right } => if let ASTNodeEnum::Variable(name) = &left.value {
				if let Some(stack_pos) = context.get_var(name) {
					self.compile(right, result, context)?;

					self.load(stack_pos, result);
				} else {
					let stack_pos = self.stack_pos;

					self.compile(right, result, context)?;

					context.insert_var(name.to_owned(), stack_pos);
				}
			} else {
				return Err(CompilerError::LeftExprShouldBeId(left.value.to_string()));
			},
			ASTNodeEnum::Binary { left, op: BinaryOp::PlusAssignment, right } =>
				self.compile_assignment(left, right, Bytecode::Add, result, context)?,
			ASTNodeEnum::Binary { left, op: BinaryOp::MinusAssignment, right } =>
				self.compile_assignment(left, right, Bytecode::Sub, result, context)?,
			ASTNodeEnum::Binary { left, op: BinaryOp::MultiplyAssignment, right } =>
				self.compile_assignment(left, right, Bytecode::Mul, result, context)?,
			ASTNodeEnum::Binary { left, op: BinaryOp::PowAssignment, right } =>
				self.compile_assignment(left, right, Bytecode::Pow, result, context)?,
			ASTNodeEnum::Binary { left, op: BinaryOp::DivideAssignment, right } =>
				self.compile_assignment(left, right, Bytecode::Div, result, context)?,
			ASTNodeEnum::Variable(name) => if let Some(stack_pos) = context.get_var(name) {
				self.look(stack_pos, result);
			} else {
				return Err(CompilerError::NotKnownAtThisScope(name.clone()));
			},
			ASTNodeEnum::Unary { op, value } => {
				self.compile(value, result, context)?;

				result.push(Bytecode::from_unary_op(op));
			},
			ASTNodeEnum::Binary { left, op: BinaryOp::LessOrEquals, right } => {
				self.compile(right, result, context)?;
				self.compile(left, result, context)?;

				self.stack_pos -= 1;
				self.stack_pos -= 1;

				result.push(Bytecode::Gt.into());
				result.push(Bytecode::Not.into());
			},
			ASTNodeEnum::Binary { left, op: BinaryOp::GreatOrEquals, right } => {
				self.compile(right, result, context)?;
				self.compile(left, result, context)?;

				self.stack_pos -= 1;
				self.stack_pos -= 1;

				result.push(Bytecode::Lt.into());
				result.push(Bytecode::Not.into());
			},
			ASTNodeEnum::Binary { left, op, right } => {
				self.compile(right, result, context)?;
				self.compile(left, result, context)?;

				self.stack_pos -= 1;
				self.stack_pos -= 1;

				result.push(Bytecode::from_binary_op(op).into());
			},
			ASTNodeEnum::Function { name, arg } if let ASTNodeEnum::Tuple(values) = &arg.value => {
				self.call_func(name, values.iter(), result, context)?;
			},
			ASTNodeEnum::Function { name, arg } => {
				self.call_func(name, iter::once(&**arg), result, context)?;
			},
			ASTNodeEnum::While { condition, block, block_else: None } => {
				let mut compiled_condition = Vec::new();
				self.compile(condition, &mut compiled_condition, context)?;

				let Ok(compiled_condition_size) = u32::try_from(compiled_condition.len()) else {
					return Err(CompilerError::BlockIsTooLong)
				};

				let mut compiled_block = Vec::new();
				self.compile(block, &mut compiled_block, context)?;

				let Ok(compiled_block_size) = u32::try_from(compiled_block.len()) else {
					return Err(CompilerError::BlockIsTooLong)
				};

				let cond = self.pc;

				result.extend(compiled_condition);

				self.jif(self.pc + compiled_block_size + 5, result);

				result.extend(compiled_block);
				self.jmp(cond, result);
			},
			ASTNodeEnum::If { condition, block, block_else: Some(block_else) } => {
				let mut compiled_block = Vec::new();
				self.compile(block, &mut compiled_block, context)?;

				let Ok(else_offset) = u32::try_from(5 + compiled_block.len()) else {
					return Err(CompilerError::BlockIsTooLong)
				};

				let mut compiled_block_else = Vec::new();
				self.compile(block_else, &mut compiled_block_else, context)?;

				let Ok(else_block_size) = u32::try_from(compiled_block_else.len()) else {
					return Err(CompilerError::BlockIsTooLong)
				};

				self.compile(condition, result, context)?;

				self.jif(self.pc + else_offset, result);

				result.extend(compiled_block);
				self.jmp(self.pc + else_block_size, result);

				result.extend(compiled_block_else);
			},
			ASTNodeEnum::If { condition, block, block_else: None } => {
				let mut compiled_block = Vec::new();

				self.compile(block, &mut compiled_block, context)?;

				let Ok(u32_len) = u32::try_from(compiled_block.len()) else {
					return Err(CompilerError::BlockIsTooLong)
				};

				self.compile(condition, result, context)?;

				self.jif(self.pc + u32_len, result);

				result.extend(compiled_block);
			},
			ASTNodeEnum::Block(statements) => {
				let statements_len = statements.len();

				let mut local_context = CompilerContext::new(Some(context));

				for (i, statement) in statements.iter().enumerate() {
					self.compile(statement, result, &mut local_context)?;
				}
			},
		}

		Ok(())
	}

	pub fn compile_loop(&mut self, ast: &ASTNode, result: &mut Vec<Bytecode>) -> Result<(), CompilerError> {
		self.compile(ast, result, &mut CompilerContext::new(None))
	}
}
