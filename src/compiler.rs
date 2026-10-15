pub mod lexer;
pub mod parser;
pub mod optimizer;

use crate::{native_func::{NativeFunctionArgsNumber, NativeFunctionID}, wmatch, wrapper::Spanned};

use std::{collections::HashMap};

use crate::{bytecode::Bytecode, compiler::parser::{BinaryOp::self, ASTNode, UnaryOp}};

#[derive(thiserror::Error, PartialEq, Clone, Debug)]
pub enum CompilerError {
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

	fn get_child_function(&self, name: &str) -> Option<u32> {
		if let Some(parent) = self.parent {
			parent.get_function(name)
		} else if let Some(&value) = self.functions.get(name) {
			Some(value)
		} else  {
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
	global_functions: HashMap<String, u32>,
}

impl Compiler {
	pub fn new() -> Self {
		Self {stack: 0, break_pos: Vec::new(), global_functions: HashMap::new()}
	}

	fn pc(result: &[Bytecode]) -> u32 {
		result.len() as u32
	}

	fn look(&mut self, stack_pos: u32, result: &mut Vec<Bytecode>) {
		result.push(Bytecode::Look(stack_pos));

		self.stack += 1;
	}

	fn load(&mut self, stack_pos: u32, result: &mut Vec<Bytecode>) {
		result.push(Bytecode::Load(stack_pos));

		self.stack -= 1;
	}

	fn call_func<'a, T>(&mut self, name: &Spanned<&str>, args: T, result: &mut Vec<Bytecode>, context: &mut CompilerContext) -> Result<(), Spanned<CompilerError>>
	where T: Iterator<Item=&'a Spanned<ASTNode>> {
		if let Ok(id) = NativeFunctionID::try_from(name.val) {
			let mut len = 0;

			for arg in args {
				self.compile(arg, result, context)?;

				len += 1;
			}

			match id.args() {
				NativeFunctionArgsNumber::Fixed(fixed) => {
					if len != fixed {
						return Err(
							Spanned::from_with_span(
								CompilerError::ArgumentsNativeFunctionCall {
									id,
									found: len,
									expected: fixed
								},
								name.span
							)
						)
					}
				}

				NativeFunctionArgsNumber::Dynamic => result.push(Bytecode::PushNumberU32(len)),
			}

			result.push(Bytecode::NativeCall(id));
			self.stack -= len;
			self.stack += 1;

			return Ok(())
		} else if let Some(pos) = context.get_function(name.val) {
			for arg in args {
				self.compile(arg, result, context)?;
			}

			result.push(Bytecode::Call(pos));

			return Ok(())
		}

		Err(
			Spanned::from_with_span(
				CompilerError::UnknownFunctionCall(name.val.to_owned()),
				name.span
			)
		)
	}

	fn compile_assignment(&mut self, left: &Spanned<ASTNode>, right: &Spanned<ASTNode>, op: Bytecode, result: &mut Vec<Bytecode>, context: &mut CompilerContext) -> Result<(), Spanned<CompilerError>> {
		let ASTNode::Variable(name) = &left.val else {
			return Err(
				Spanned::from_with_span(
					CompilerError::LeftExprShouldBeId(left.val.to_string()),
					left.span
				)
			);
		};

		let Some(stack_pos) = context.get_var(name) else {
			return Err(
				Spanned::from_with_span(
					CompilerError::NotKnownAtThisScope(name.to_owned()),
					left.span
				)
			);
		};

		self.look(stack_pos, result);
		self.compile(right, result, context)?;

		self.stack -= 1;

		result.push(op);

		self.load(stack_pos, result);

		Ok(())
	}

	fn compile(&mut self, ast: &Spanned<ASTNode>, result: &mut Vec<Bytecode>, context: &mut CompilerContext) -> Result<(), Spanned<CompilerError>> {
		println!("context = {context:?}");

		println!("stack_pos = {}; ast = {ast}", self.stack);

		let span = ast.span;

		match &ast.val {
			ASTNode::FunctionDefinition { name: Some(name), args, body, is_global } => {
				let jmp = Self::pc(result);
				result.push(Bytecode::Jmp(0));

				context.insert_function(name.val.clone(), Self::pc(result));

				result.push(Bytecode::Subprogram);

				let mut body_context = CompilerContext::new(Some(context));

				for (i, arg) in args.iter().enumerate().rev() {
					self.stack += 1;

					body_context.insert_var(&arg.val, i as u32);
				}

				self.compile(body, result, &mut body_context)?;

				result.push(Bytecode::Ret);

				*result.get_mut(jmp as usize).expect("result.len() < jmp") = Bytecode::Jmp(Self::pc(result));
			},
			ASTNode::Return(value) => {
				self.compile(value, result, context)?;

				result.push(Bytecode::Ret);
			},
			ASTNode::Break(value) => {
				self.compile(value, result, context)?;

				let Some(pos) = self.break_pos.last_mut() else {
					return Err(
						Spanned::from_with_span(CompilerError::BreakOutsideOfLoop, span)
					)
				};

				*pos = Some(Self::pc(result));

				result.push(Bytecode::Jmp(0));
			},
			ASTNode::FunctionDefinition {..} => todo!("{}", ast.val),
			ASTNode::None => return Ok(()),
			&ASTNode::Boolean(x) => {
				self.stack += 1;

				result.push(Bytecode::PushBoolean(x));
			},
			&ASTNode::Float(x) => {
				self.stack += 1;

				result.push(Bytecode::PushFloat(x));
			},
			ASTNode::String(x) => {
				self.stack += 1;

				result.push(Bytecode::PushString(x.clone()));
			},
			ASTNode::List(values) => if !values.is_empty() {
				self.call_func(
					&Spanned::from_with_span("list", span),
					values.iter(), result, context
				)?;
			},
			ASTNode::Binary { left, op: wmatch!(BinaryOp::Range), right } => {
				self.call_func(
					&Spanned::from_with_span("range", left.span),
					[&**left, &**right].into_iter(), result, context
				)?;
			},
			ASTNode::Binary { left, op: wmatch!(BinaryOp::Assignment), right } => {
				let ASTNode::Variable(name) = &left.val else {
					return Err(
						Spanned::from_with_span(
							CompilerError::LeftExprShouldBeId(left.val.to_string()),
							span
						)
					);
				};

				if let Some(stack_pos) = context.get_var(name) {
					self.compile(right, result, context)?;

					self.load(stack_pos, result);
				} else {
					let stack_pos = self.stack;

					self.compile(right, result, context)?;

					context.insert_var(name.to_owned(), stack_pos);
				}
			},
			ASTNode::Binary { left, op: wmatch!(BinaryOp::PlusAssignment), right } => {
				self.compile_assignment(left, right, Bytecode::Add, result, context)?;
			},
			ASTNode::Binary { left, op: wmatch!(BinaryOp::MinusAssignment), right } => {
				self.compile_assignment(left, right, Bytecode::Sub, result, context)?;
			},
			ASTNode::Binary { left, op: wmatch!(BinaryOp::MultiplyAssignment), right } => {
				self.compile_assignment(left, right, Bytecode::Mul, result, context)?;
			},
			ASTNode::Binary { left, op: wmatch!(BinaryOp::PowAssignment), right } => {
				self.compile_assignment(left, right, Bytecode::Pow, result, context)?;
			},
			ASTNode::Binary { left, op: wmatch!(BinaryOp::DivideAssignment), right } => {
				self.compile_assignment(left, right, Bytecode::Div, result, context)?;
			},
			ASTNode::Variable(name) => {
				let Some(stack_pos) = context.get_var(name) else {
					return Err(
						Spanned::from_with_span(
							CompilerError::NotKnownAtThisScope(name.clone()),
							span
						)
					);
				};

				self.look(stack_pos, result);
			},
			ASTNode::Unary { op, value } => {
				self.compile(value, result, context)?;

				result.push(Bytecode::from_unary_op(op));
			},
			ASTNode::Binary { left, op: wmatch!(BinaryOp::LessOrEquals), right } => {
				self.compile(right, result, context)?;
				self.compile(left, result, context)?;

				self.stack -= 1;

				result.push(Bytecode::from_binary_op(&BinaryOp::Less));
				result.push(Bytecode::from_unary_op(&UnaryOp::Not));
			},
			ASTNode::Binary { left, op: wmatch!(BinaryOp::Great), right } => {
				self.compile(right, result, context)?;
				self.compile(left, result, context)?;

				self.stack -= 1;

				result.push(Bytecode::from_binary_op(&BinaryOp::Less));
			},
			ASTNode::Binary { left, op, right } => {
				self.compile(left, result, context)?;
				self.compile(right, result, context)?;

				self.stack -= 1;

				result.extend(Bytecode::bytecodes_from_binary_op(&op.val));
			},
			ASTNode::Function { name, args } => {
				self.call_func(
					&Spanned::from_with_span(name, span),
					args.iter(), result, context
				)?;
			},
			ASTNode::While { condition, body, else_body: Some(else_body) } => {
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
			ASTNode::While { condition, body, else_body: None } => {
				let cond = Self::pc(result);

				self.compile(condition, result, context)?;

				let jif = Self::pc(result);
				result.push(Bytecode::Jif(0));
				self.stack -= 1;

				self.break_pos.push(None);

				self.compile(body, result, context)?;

				result.push(Bytecode::Jmp(cond));

				*result.get_mut(jif as usize).expect("result.len() < jif") = Bytecode::Jif(Self::pc(result));

				if let Some(break_pos) = self.break_pos.pop().expect("Unexpected pop from self.break_pos") {
					*result.get_mut(break_pos as usize).expect("result.len() < jmp") = Bytecode::Jmp(Self::pc(result));
				}
			},
			ASTNode::If { condition, body, else_body: Some(else_body) } => {
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
			ASTNode::If { condition, body, else_body: None } => {
				self.compile(condition, result, context)?;

				self.stack -= 1;
				let jif = Self::pc(result);

				result.push(Bytecode::Jif(0));

				self.compile(body, result, context)?;

				*result.get_mut(jif as usize).expect("result.len() < jif") = Bytecode::Jif(Self::pc(result));
			},
			ASTNode::Block(statements) => {
				let mut local_context = CompilerContext::new(Some(context));

				for statement in statements {
					self.compile(statement, result, &mut local_context)?;
				}
			},
		}

		Ok(())
	}

	pub fn compile_loop(&mut self, ast: &Spanned<ASTNode>, result: &mut Vec<Bytecode>) -> Result<(), Spanned<CompilerError>> {
		let mut context = CompilerContext::new(None);

		self.compile(ast, result, &mut context)?;

		self.call_func(
			&Spanned::from_with_span("main", 0),
			[].iter(),
			result,
			&mut context
		)?;

		Ok(())
	}
}
