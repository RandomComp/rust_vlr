use std::{collections::HashMap, fmt::Display, iter, num::TryFromIntError};

use crate::{parser::{ASTNode, ASTNodeEnum, BinaryOp, UnaryOp}, vm::VM};

#[derive(PartialEq)]
pub enum Bytecode {
	Nop, // Do nothing
	PushFloat, // Pushes number
	PushString, // Pushes string to stack (needing len)
	PushFalse, // Pushes false to stack
	PushTrue, // Pushes true to stack
	Look, // Copy value from N
	Load, // Copy value to N
	Remove, // Remove value
	PushArgEnd,
	Jit, // Jump if condition
	Jif, // Jump if not condition
	Jmp, // Jump without condition
	Call, // Jump to pc from stack
	Ret, // Jump to pc from stack
	Eq, // Equals
	Neq, // Not equals
	Lt, // Less than
	Gt, // Great than
	NativeCall,
	Add, // Additive
	Sub, // Subtraction
	Mul, // Multiplication
	Div, // Division
	Pow, // Power of
	Neg, // Negative value (-)
	Not, // Logical not
	LogicalAnd, // Logical AND
	LogicalOr, // Logical OR
}

impl TryFrom<u8> for Bytecode {
	type Error = ();

	fn try_from(value: u8) -> Result<Self, Self::Error> {
		let result = match value {
			0x00 | 0xFF => Self::Nop, // Do nothing
			0x01 => Self::PushFloat, // Pushes number
			0x02 => Self::PushString, // Pushes string to stack (needing len)
			0x03 => Self::PushFalse, // Pushes false to stack
			0x04 => Self::PushTrue, // Pushes true to stack
			0x05 => Self::Look, // Copy value from N
			0x06 => Self::Load, // Copy value to N
			0x07 => Self::Remove, // Remove value
			0x0F => Self::PushArgEnd,
			0x20 => Self::Jit, // Jump if condition
			0x21 => Self::Jif, // Jump if not condition
			0x28 => Self::Jmp, // Jump without condition
			0x2A => Self::Call, // Jump to pc from stack
			0x2B => Self::Ret, // Jump to pc from stack
			0x50 => Self::Eq, // Equals
			0x51 => Self::Neq, // Not equals
			0x52 => Self::Lt, // Less than
			0x53 => Self::Gt, // Great than
			0x80 => Self::NativeCall,
			0xA0 => Self::Add, // Additive
			0xA3 => Self::Sub, // Subtraction
			0xB0 => Self::Mul, // Multiplication
			0xB3 => Self::Div, // Division
			0xC0 => Self::Pow, // Power of
			0xC3 => Self::Neg, // Negative value (-)
			0xD0 => Self::Not, // Logical not
			0xD3 => Self::LogicalAnd, // Logical AND
			0xE0 => Self::LogicalOr, // Logical OR
			_ => return Err(()),
		};

		Ok(result)
	}
}

impl From<&Bytecode> for u8 {
	fn from(value: &Bytecode) -> Self {
		match value {
			Bytecode::Nop 			=> 0x00, // Do nothing
			Bytecode::PushFloat 	=> 0x01, // Pushes number
			Bytecode::PushString 	=> 0x02, // Pushes string to stack (needing len)
			Bytecode::PushFalse 	=> 0x03, // Pushes false to stack
			Bytecode::PushTrue 		=> 0x04, // Pushes true to stack
			Bytecode::Look 			=> 0x05, // Copy value from N
			Bytecode::Load 			=> 0x06, // Copy value to N
			Bytecode::Remove 		=> 0x07, // Remove value
			Bytecode::PushArgEnd 	=> 0x0F,
			Bytecode::Jit 			=> 0x20, // Jump if condition
			Bytecode::Jif 			=> 0x21, // Jump if not condition
			Bytecode::Jmp 			=> 0x28, // Jump without condition
			Bytecode::Call 			=> 0x2A, // Jump to pc from stack
			Bytecode::Ret 			=> 0x2B, // Jump to pc from stack
			Bytecode::Eq 			=> 0x50, // Equals
			Bytecode::Neq 			=> 0x51, // Not equals
			Bytecode::Lt 			=> 0x52, // Less than
			Bytecode::Gt 			=> 0x53, // Great than
			Bytecode::NativeCall 	=> 0x80,
			Bytecode::Add 			=> 0xA0, // Additive
			Bytecode::Sub 			=> 0xA3, // Subtraction
			Bytecode::Mul 			=> 0xB0, // Multiplication
			Bytecode::Div 			=> 0xB3, // Division
			Bytecode::Pow 			=> 0xC0, // Power of
			Bytecode::Neg 			=> 0xC3, // Negative value (-)
			Bytecode::Not 			=> 0xD0, // Logical not
			Bytecode::LogicalAnd 	=> 0xD3, // Logical AND
			Bytecode::LogicalOr 	=> 0xE0, // Logical OR
		}
	}
}

impl From<Bytecode> for u8 {
	fn from(value: Bytecode) -> Self {
		u8::from(&value)
	}
}

impl Bytecode {
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

impl Display for Bytecode {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let value: u8 = self.into();

    	write!(f, "{:2X}", value)
	}
}

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

pub fn get_fn_name<F>() -> &'static str {
	std::any::type_name::<F>()
}

pub struct Compiler {
	stack_pos: u32,
	pc: u32,
}

impl Compiler {
	pub fn new() -> Self {
		Self {stack_pos: 0, pc: 0}
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

		result.push(Bytecode::Look.into());
		result.extend(stack_pos.to_le_bytes());

		self.pc += 1 + 4;
	}

	fn load(&mut self, stack_pos: u32, result: &mut Vec<u8>) {
		result.push(Bytecode::Load.into());
		result.extend(stack_pos.to_le_bytes());

		self.pc += 1 + 4;
	}

	fn push_bool(&mut self, val: bool, result: &mut Vec<u8>) {
		self.stack_pos += 1;

		if val {
			result.push(Bytecode::PushTrue.into());
		} else {
			result.push(Bytecode::PushFalse.into());
		}

		self.pc += 1;
	}

	fn push_float(&mut self, val: f64, result: &mut Vec<u8>) {
		self.stack_pos += 1;

		result.push(Bytecode::PushFloat.into());
		result.extend(val.to_le_bytes());

		self.pc += 1 + 8;
	}

	fn push_string(&mut self, val: &str, result: &mut Vec<u8>) -> Result<(), CompilerError> {
		self.stack_pos += 1;

		let Ok(len_u32) = u32::try_from(val.len()) else {
			return Err(CompilerError::StringTooLong)
		};

		result.push(Bytecode::PushString.into());
		result.extend(len_u32.to_le_bytes());
		result.extend(val.bytes());

		self.pc += 1 + 2 + len_u32;

		Ok(())
	}

	fn push_arg_end(&mut self, result: &mut Vec<u8>) {
		self.stack_pos += 1;

		result.push(Bytecode::PushArgEnd.into());

		self.pc += 1;
	}

	fn call_func<'a, T>(&mut self, name: &str, args: T, result: &mut Vec<u8>, context: &mut CompilerContext) -> Result<(), CompilerError> where T: DoubleEndedIterator<Item=&'a ASTNode> {
		if let Some(id) = Self::get_native_call_id_by_name(name) {
			self.push_arg_end(result);

			let mut len = 1;

			for arg in args.rev() {
				self.compile(arg, result, context)?;

				len += 1;
			}

			self.stack_pos -= len;

			result.push(Bytecode::NativeCall.into());
			result.extend(id.to_le_bytes());

			self.pc += 1 + 1;
		} else if let Some(pos) = context.get_function(name) {
			self.call(pos, result);
		} else {
			return Err(CompilerError::UnknownFunctionCall(name.to_owned()))
		}

		Ok(())
	}

	fn jit(&mut self, pos: u32, result: &mut Vec<u8>) {
		self.stack_pos -= 1;

		result.push(Bytecode::Jit.into());
		result.extend(pos.to_le_bytes());

		self.pc += 1 + 4;
	}

	fn jif(&mut self, pos: u32, result: &mut Vec<u8>) {
		self.stack_pos -= 1;

		result.push(Bytecode::Jif.into());
		result.extend(pos.to_le_bytes());

		self.pc += 1 + 4;
	}

	fn jmp(&mut self, pos: u32, result: &mut Vec<u8>) {
		result.push(Bytecode::Jmp.into());
		result.extend(pos.to_le_bytes());

		self.pc += 1 + 4;
	}

	fn call(&mut self, pos: u32, result: &mut Vec<u8>) {
		result.push(Bytecode::Call.into());
		result.extend(pos.to_le_bytes());

		self.pc += 1 + 4;
	}

	fn ret(&mut self, result: &mut Vec<u8>) {
		result.push(Bytecode::Ret.into());

		self.pc += 1;
	}

	fn compile_assignment(&mut self, left: &ASTNode, right: &ASTNode, op: Bytecode, result: &mut Vec<u8>, context: &mut CompilerContext) -> Result<(), CompilerError> {
		let ASTNodeEnum::Variable(name) = &left.value else {
			return Err(CompilerError::LeftExprShouldBeId(left.value.to_string()));
		};

		let Some(stack_pos) = context.get_var(name) else {
			return Err(CompilerError::NotKnownAtThisScope(name.to_owned()));
		};

		self.compile(right, result, context)?;
		self.look(stack_pos, result);

		result.push(op.into());
		self.pc += 1;

		self.load(stack_pos, result);

		Ok(())
	}

	fn compile(&mut self, ast: &ASTNode, result: &mut Vec<u8>, context: &mut CompilerContext) -> Result<(), CompilerError> {
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
				self.push_string(x, result)?;
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

				result.push(Bytecode::from_unary_op(op).into());
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

				println!("condition = {condition}");

				VM::new(Some(result.clone())).disasm().unwrap();

				println!("self.stack_pos = {}", self.stack_pos);

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

	pub fn compile_loop(&mut self, ast: &ASTNode, result: &mut Vec<u8>) -> Result<(), CompilerError> {
		self.compile(ast, result, &mut CompilerContext::new(None))
	}
}
