use std::{collections::HashMap, fmt::Display, iter};

use crate::{bytecode::Bytecode, parser::{ASTNode, ASTNodeEnum, BinaryOp}};

#[derive(thiserror::Error, Debug)]
pub enum CompilerError {
	#[error("Unknown function '{0}' call")]
	UnknownFunctionCall(String),
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
}

impl Compiler {
	pub fn new() -> Self {
		Self {stack_pos: 0}
	}

	fn pc(result: &[Bytecode]) -> u32 {
		result.len() as u32
	}

	fn look(&mut self, stack_pos: u32, result: &mut Vec<Bytecode>) {
		self.stack_pos += 1;

		result.push(Bytecode::Look(stack_pos));
	}

	fn load(&mut self, stack_pos: u32, result: &mut Vec<Bytecode>) {
		self.stack_pos -= 1;

		result.push(Bytecode::Load(stack_pos));
	}

	fn call_func<'a, T>(&mut self, name: &str, args: T, result: &mut Vec<Bytecode>, context: &mut CompilerContext) -> Result<u32, CompilerError> where T: DoubleEndedIterator<Item=&'a ASTNode> {
		let pc = Self::pc(result);

		if let Ok(id) = NativeFunctionId::try_from(name) {
			self.stack_pos += 1;

			result.push(Bytecode::PushArgEnd);

			let mut len = 1;

			for arg in args.rev() {
				self.compile(arg, result, context)?;

				len += 1;
			}

			self.stack_pos -= len;

			result.push(Bytecode::NativeCall(id));
		} else if let Some(pos) = context.get_function(name) {
			result.push(Bytecode::Call(pos));
		} else {
			return Err(CompilerError::UnknownFunctionCall(name.to_owned()))
		}

		Ok(pc)
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

		self.load(stack_pos, result);

		Ok(())
	}

	fn compile(&mut self, ast: &ASTNode, result: &mut Vec<Bytecode>, context: &mut CompilerContext) -> Result<(), CompilerError> {
		println!("stack_pos = {}; ast = {ast}", self.stack_pos);

		match &ast.value {
			ASTNodeEnum::FunctionDefinition { name: Some(name), args, block } => {
				let mut compiled_block = Vec::new();

				self.compile(block, &mut compiled_block, context)?;

				let Ok(compiled_block_len) = u32::try_from(compiled_block.len()) else {
					return Err(CompilerError::BlockIsTooLong)
				};

				result.push(Bytecode::Jmp(0 + compiled_block_len + 6));

				context.insert_function(name.to_owned(), 0);

				result.extend(compiled_block);

				result.push(Bytecode::Ret);
			},
			ASTNodeEnum::While { block_else: Some(_), .. } |
			ASTNodeEnum::Break(..) | ASTNodeEnum::Return(..) |
			ASTNodeEnum::FunctionDefinition {..} => todo!("{}", ast.value),
			ASTNodeEnum::None => return Ok(()),
			&ASTNodeEnum::Boolean(x) => {
				self.stack_pos += 1;

				result.push(Bytecode::PushBoolean(x));
			},
			&ASTNodeEnum::Float(x) => {
				self.stack_pos += 1;

				result.push(Bytecode::PushFloat(x));
			},
			ASTNodeEnum::String(x) => {
				self.stack_pos += 1;

				result.push(Bytecode::PushString(x.clone()));
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
			ASTNodeEnum::Binary { left, op: BinaryOp::PlusAssignment, right } => {
				self.compile_assignment(left, right, Bytecode::Add, result, context)?;
			},
			ASTNodeEnum::Binary { left, op: BinaryOp::MinusAssignment, right } => {
				self.compile_assignment(left, right, Bytecode::Sub, result, context)?;
			},
			ASTNodeEnum::Binary { left, op: BinaryOp::MultiplyAssignment, right } => {
				self.compile_assignment(left, right, Bytecode::Mul, result, context)?;
			},
			ASTNodeEnum::Binary { left, op: BinaryOp::PowAssignment, right } => {
				self.compile_assignment(left, right, Bytecode::Pow, result, context)?;
			},
			ASTNodeEnum::Binary { left, op: BinaryOp::DivideAssignment, right } => {
				self.compile_assignment(left, right, Bytecode::Div, result, context)?;
			},
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

				result.push(Bytecode::Gt);
				result.push(Bytecode::Not);
			},
			ASTNodeEnum::Binary { left, op: BinaryOp::GreatOrEquals, right } => {
				self.compile(right, result, context)?;
				self.compile(left, result, context)?;

				self.stack_pos -= 1;

				result.push(Bytecode::Lt);
				result.push(Bytecode::Not);
			},
			ASTNodeEnum::Binary { left, op, right } => {
				self.compile(right, result, context)?;
				self.compile(left, result, context)?;

				self.stack_pos -= 1;

				result.push(Bytecode::from_binary_op(op));
			},
			ASTNodeEnum::Function { name, arg } if let ASTNodeEnum::Tuple(values) = &arg.value => {
				self.call_func(name, values.iter(), result, context)?;
			},
			ASTNodeEnum::Function { name, arg } => {
				self.call_func(name, iter::once(&**arg), result, context)?;
			},
			ASTNodeEnum::While { condition, block, block_else: None } => {
				let cond = Self::pc(result);

				self.compile(condition, result, context)?;

				let jif = Self::pc(result);

				self.compile(block, result, context)?;

				result.push(Bytecode::Jmp(cond));

				result.insert(jif as usize, Bytecode::Jif(Self::pc(result) + 1));
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

				self.stack_pos -= 1;
				result.push(Bytecode::Jif(0 + else_offset));

				result.extend(compiled_block);
				result.push(Bytecode::Jmp(0 + else_block_size));

				result.extend(compiled_block_else);
			},
			ASTNodeEnum::If { condition, block, block_else: None } => {
				self.compile(condition, result, context)?;

				self.stack_pos -= 1;
				let jif = Self::pc(result);

				self.compile(block, result, context)?;

				result.insert(jif as usize, Bytecode::Jif(Self::pc(result) + 1));
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
