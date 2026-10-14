pub mod lexer;
pub mod parser;
pub mod optimizer;

use crate::native_func::{NativeFunctionArgsNumber, NativeFunctionID};

use std::{collections::HashMap, iter};

use crate::{bytecode::Bytecode, compiler::parser::{BinaryOp::self, RawASTNode, UnaryOp}, wrapper::{ASTNode, CompilerError}};

#[derive(thiserror::Error, PartialEq, Clone, Debug)]
pub enum RawCompilerError {
	#[error("Unknown function '{0}' call")]
	UnknownFunctionCall(String),
	#[error("Too few arguments for native function '{id}' call, expected {expected}, found {found}")]
	ArgumentsNativeFunctionCall {id: NativeFunctionID, found: u32, expected: u32 },
	#[error("'{0}' variable is not known in current scope")]
	NotKnownAtThisScope(String),
	#[error("L-value of assignment should be variable name, not '{0}'")]
	LeftExprShouldBeId(String),
	#[error("Break outside of loop")]
	BreakOutsideOfLoop,
	#[error("Return outside of function")]
	ReturnOutsideOfFunction,
}

#[derive(Debug)]
struct CompilerContext<'a> {
	functions: HashMap<String, u32>,
	vars: HashMap<String, u32>,
	parent: Option<&'a CompilerContext<'a>>,
}

impl<'a> CompilerContext<'a> {
	fn new(parent: Option<&'a Self>) -> Self {
		Self {vars: HashMap::new(), functions: HashMap::new(), parent}
	}

	fn get_var(&self, name: &str) -> Option<u32> {
		if let Some(&value) = self.vars.get(name) {
			Some(value)
		} else if let Some(parent) = self.parent {
			parent.get_var(name)
		} else {
			None
		}
	}
	fn get_function(&self, name: &str) -> Option<u32> {
		if let Some(&value) = self.functions.get(name) {
			Some(value)
		} else if let Some(parent) = self.parent {
			parent.get_function(name)
		} else {
			None
		}
	}

	fn insert_var<T>(&mut self, name: T, stack_pos: u32) where T: Into<String> {
		self.vars.insert(name.into(), stack_pos);
	}
	fn insert_function<T>(&mut self, name: T, pos: u32) where T: Into<String> {
		self.functions.insert(name.into(), pos);
	}
}

pub struct Compiler {
	stack: u32,
	break_pos: Vec<Option<u32>>,
}

impl Compiler {
	pub fn new() -> Self {
		Self {stack: 0, break_pos: Vec::new()}
	}

	fn pc(result: &[Bytecode]) -> u32 {
		result.len() as u32
	}

	fn look(&mut self, stack_pos: u32, result: &mut Vec<Bytecode>) {
		result.push(Bytecode::Look(self.stack - stack_pos - 1));

		self.stack += 1;
	}

	fn load(&mut self, stack_pos: u32, result: &mut Vec<Bytecode>) {
		result.push(Bytecode::Load(self.stack - stack_pos - 1));

		self.stack -= 1;
	}

	fn call_func<'a, T>(&mut self, name: &str, args: T, result: &mut Vec<Bytecode>, context: &mut CompilerContext) -> Result<(), RawCompilerError>
	where T: Iterator<Item=&'a ASTNode> {
		if let Ok(id) = NativeFunctionID::try_from(name) {
			let mut len = 0;

			for arg in args {
				self.compile(arg, result, context)?;

				len += 1;
			}

			match id.args() {
				NativeFunctionArgsNumber::Fixed(fixed) => {
					if len != fixed {
						return Err(
							RawCompilerError::ArgumentsNativeFunctionCall {
								id,
								found: len,
								expected: fixed
							}
						)
					}
				}

				NativeFunctionArgsNumber::Dynamic => result.push(Bytecode::PushNumberU32(len)),
			}

			result.push(Bytecode::NativeCall(id));
			self.stack -= len;
			self.stack += 1;
		} else if let Some(pos) = context.get_function(name) {
			for arg in args {
				self.compile(arg, result, context)?;
			}

			result.push(Bytecode::Call(pos));
		} else {
			return Err(RawCompilerError::UnknownFunctionCall(name.to_owned()))
		}

		Ok(())
	}

	fn compile_assignment(&mut self, left: &ASTNode, right: &ASTNode, op: Bytecode, result: &mut Vec<Bytecode>, context: &mut CompilerContext) -> Result<(), RawCompilerError> {
		let RawASTNode::Variable(name) = &left.val else {
			return Err(RawCompilerError::LeftExprShouldBeId(left.val.to_string()));
		};

		let Some(stack_pos) = context.get_var(name) else {
			return Err(RawCompilerError::NotKnownAtThisScope(name.to_owned()));
		};

		self.look(stack_pos, result);
		self.compile(right, result, context)?;

		self.stack -= 1;

		result.push(op);

		self.load(stack_pos, result);

		Ok(())
	}

	fn compile(&mut self, ast: &ASTNode, result: &mut Vec<Bytecode>, context: &mut CompilerContext) -> Result<(), CompilerError> {
		println!("context = {context:?}");

		println!("stack_pos = {}; ast = {ast}", self.stack);

		let span = ast.span;

		match &ast.val {
			RawASTNode::FunctionDefinition { name: Some(name), args, body } => {
				let jmp = Self::pc(result);
				result.push(Bytecode::Jmp(0));

				context.insert_function(name.to_owned(), Self::pc(result));

				result.push(Bytecode::Subprogram);

				let mut body_context = CompilerContext::new(Some(context));

				for (i, arg) in args.iter().enumerate().rev() {
					self.stack += 1;

					body_context.insert_var(arg, i as u32);
				}

				self.compile(body, result, &mut body_context)?;

				result.push(Bytecode::Ret);

				*result.get_mut(jmp as usize).expect("result.len() < jmp") = Bytecode::Jmp(Self::pc(result));
			},
			RawASTNode::Return(value) => {
				self.compile(value, result, context)?;

				result.push(Bytecode::Ret);
			},
			RawASTNode::Break(value) => {
				self.compile(value, result, context)?;

				let Some(pos) = self.break_pos.last_mut() else {
					return Err(CompilerError::from(RawCompilerError::BreakOutsideOfLoop).with_pos(span))
				};

				*pos = Some(Self::pc(result));

				result.push(Bytecode::Jmp(0));
			},
			RawASTNode::FunctionDefinition {..} => todo!("{}", ast.val),
			RawASTNode::None => return Ok(()),
			&RawASTNode::Boolean(x) => {
				self.stack += 1;

				result.push(Bytecode::PushBoolean(x));
			},
			&RawASTNode::Float(x) => {
				self.stack += 1;

				result.push(Bytecode::PushFloat(x));
			},
			RawASTNode::String(x) => {
				self.stack += 1;

				result.push(Bytecode::PushString(x.clone()));
			},
			RawASTNode::List(values) => if !values.is_empty() {
				self.call_func("list", values.iter(), result, context).map_err(|e|
					CompilerError::from(e).with_pos(span)
				)?;
			},
			RawASTNode::Binary { left, op: BinaryOp::Range, right } => {
				self.call_func("range", [&**left, &**right].into_iter(), result, context).map_err(|e|
					CompilerError::from(e).with_pos(span)
				)?;
			},
			RawASTNode::Binary { left, op: BinaryOp::Assignment, right } => {
				if let RawASTNode::Variable(name) = &left.val {
					if let Some(stack_pos) = context.get_var(name) {
						self.compile(right, result, context)?;

						self.load(stack_pos, result);
					} else {
						let stack_pos = self.stack;

						self.compile(right, result, context)?;

						context.insert_var(name.to_owned(), stack_pos);
					}
				} else {
					return Err(
						CompilerError::from(
							RawCompilerError::LeftExprShouldBeId(left.val.to_string())
						).with_pos(span)
					);
				}
			},
			RawASTNode::Binary { left, op: BinaryOp::PlusAssignment, right } => {
				self.compile_assignment(left, right, Bytecode::Add, result, context)?;
			},
			RawASTNode::Binary { left, op: BinaryOp::MinusAssignment, right } => {
				self.compile_assignment(left, right, Bytecode::Sub, result, context)?;
			},
			RawASTNode::Binary { left, op: BinaryOp::MultiplyAssignment, right } => {
				self.compile_assignment(left, right, Bytecode::Mul, result, context)?;
			},
			RawASTNode::Binary { left, op: BinaryOp::PowAssignment, right } => {
				self.compile_assignment(left, right, Bytecode::Pow, result, context)?;
			},
			RawASTNode::Binary { left, op: BinaryOp::DivideAssignment, right } => {
				self.compile_assignment(left, right, Bytecode::Div, result, context)?;
			},
			RawASTNode::Variable(name) => if let Some(stack_pos) = context.get_var(name) {
				self.look(stack_pos, result);
			} else {
				return Err(
					CompilerError::from(
						RawCompilerError::NotKnownAtThisScope(name.clone())
					).with_pos(span)
				);
			},
			RawASTNode::Unary { op, value } => {
				self.compile(value, result, context)?;

				result.push(Bytecode::from_unary_op(op));
			},
			RawASTNode::Binary { left, op: BinaryOp::LessOrEquals, right } => {
				self.compile(right, result, context)?;
				self.compile(left, result, context)?;

				self.stack -= 1;

				result.push(Bytecode::from_binary_op(&BinaryOp::Less));
				result.push(Bytecode::from_unary_op(&UnaryOp::Not));
			},
			RawASTNode::Binary { left, op: BinaryOp::Great, right } => {
				self.compile(right, result, context)?;
				self.compile(left, result, context)?;

				self.stack -= 1;

				result.push(Bytecode::from_binary_op(&BinaryOp::Less));
			},
			RawASTNode::Binary { left, op, right } => {
				self.compile(left, result, context)?;
				self.compile(right, result, context)?;

				self.stack -= 1;

				result.extend(Bytecode::bytecodes_from_binary_op(op));
			},
			RawASTNode::Function { name, args } => {
				self.call_func(name, args.iter(), result, context)?;
			},
			RawASTNode::While { condition, body, else_body: Some(else_body) } => {
				let cond = Self::pc(result);

				self.compile(condition, result, context)?;

				let jif = Self::pc(result);
				result.push(Bytecode::Jif(0));

				self.break_pos.push(None);

				self.compile(body, result, context)?;

				result.push(Bytecode::Jmp(cond));

				*result.get_mut(jif as usize).expect("result.len() < jif") = Bytecode::Jif(Self::pc(result));

				self.compile(else_body, result, context)?;

				if let Some(break_pos) = self.break_pos.pop().expect("Unexpected pop from self.break_pos") {
					*result.get_mut(break_pos as usize).expect("result.len() < jmp") = Bytecode::Jmp(Self::pc(result));
				}
			},
			RawASTNode::While { condition, body, else_body: None } => {
				let cond = Self::pc(result);

				self.compile(condition, result, context)?;

				let jif = Self::pc(result);
				result.push(Bytecode::Jif(0));

				self.break_pos.push(None);

				self.compile(body, result, context)?;

				result.push(Bytecode::Jmp(cond));

				*result.get_mut(jif as usize).expect("result.len() < jif") = Bytecode::Jif(Self::pc(result));

				if let Some(break_pos) = self.break_pos.pop().expect("Unexpected pop from self.break_pos") {
					*result.get_mut(break_pos as usize).expect("result.len() < jmp") = Bytecode::Jmp(Self::pc(result));
				}
			},
			RawASTNode::If { condition, body, else_body: Some(else_body) } => {
				self.compile(condition, result, context)?;

				self.stack -= 1;
				let jif = Self::pc(result);
				result.push(Bytecode::Jif(0));

				self.compile(body, result, context)?;
				let jmp = Self::pc(result);
				result.push(Bytecode::Jmp(0));

				*result.get_mut(jif as usize).expect("result.len() < jif") = Bytecode::Jif(Self::pc(result));

				self.compile(else_body, result, context)?;

				*result.get_mut(jmp as usize).expect("result.len() < jif") = Bytecode::Jmp(Self::pc(result));
			},
			RawASTNode::If { condition, body, else_body: None } => {
				self.compile(condition, result, context)?;

				self.stack -= 1;
				let jif = Self::pc(result);

				result.push(Bytecode::Jif(0));

				self.compile(body, result, context)?;

				*result.get_mut(jif as usize).expect("result.len() < jif") = Bytecode::Jif(Self::pc(result));
			},
			RawASTNode::Block(statements) => {
				let mut local_context = CompilerContext::new(Some(context));

				for statement in statements {
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
