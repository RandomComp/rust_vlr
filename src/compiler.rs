pub mod lexer;
pub mod parser;
pub mod optimizer;
pub mod native_func;

use std::fmt;

use crate::assembler::{BytecodeRegister, defs::BytecodeError, optimizer::AssemblerOptimizer};
use crate::compiler::native_func::{NativeFunctionArgsNumber, NativeFunctionID};
use crate::vm::VMWord;
use crate::wrapper::{wmatch, Spanned};

use std::{collections::HashMap};

use crate::{assembler::Bytecode, compiler::parser::{BinaryOp::self, ASTNode, UnaryOp}};

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

	/// Code segment
	code_seg: Vec<Bytecode>,
	/// Read-only data segment
	data_seg: Vec<u8>,
}

impl Compiler {
	pub fn new() -> Self {
		Self {
			stack: 0,
			break_pos: Vec::new(),
			global_functions: HashMap::new(),

			code_seg: Vec::new(),
			data_seg: Vec::new(),
		}
	}

	fn pc(&self) -> VMWord {
		self.code_seg.len() as VMWord
	}

	fn result_push(&mut self, code: Bytecode) {
		self.code_seg.push(code.into());
	}

	fn look_to_reg(&mut self, reg: BytecodeRegister, stack_pos: VMWord) {
		self.result_push(Bytecode::LoadToReg(reg, BytecodeRegister::StackPointer, stack_pos));

		self.stack += 1;
	}

	fn load_from_reg(&mut self, reg: BytecodeRegister, stack_pos: VMWord) {
		self.result_push(Bytecode::LoadFromReg(reg, BytecodeRegister::StackPointer, stack_pos));

		self.stack -= 1;
	}

	fn call_func<'a, T>(&mut self, name: &Spanned<&str>, args: T, context: &mut CompilerContext) -> Result<(), Spanned<CompilerError>>
	where T: Iterator<Item=&'a Spanned<ASTNode>> {
		if let Ok(id) = NativeFunctionID::try_from(name.val) {
			let mut len = 0;

			for arg in args {
				self.compile(arg, context)?;

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

				NativeFunctionArgsNumber::Dynamic =>
					self.result_push(Bytecode::PushNumber(len)),
			}

			self.result_push(Bytecode::NativeCall(id));
			self.stack -= len;
			self.stack += 1;

			return Ok(())
		} else if let Some(&pos) = self.global_functions.get(name.val) {
			for arg in args {
				self.compile(arg, context)?;
			}

			self.result_push(Bytecode::Call(pos));

			return Ok(())
		} else if let Some(pos) = context.get_function(name.val) {
			for arg in args {
				self.compile(arg, context)?;
			}

			self.result_push(Bytecode::Call(pos));

			return Ok(())
		}

		Err(
			Spanned::from_with_span(
				CompilerError::UnknownFunctionCall(name.val.to_owned()),
				name.span
			)
		)
	}

	fn compile_assignment(&mut self, left: &Spanned<ASTNode>, right: &Spanned<ASTNode>, op: Bytecode, context: &mut CompilerContext) -> Result<(), Spanned<CompilerError>> {
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

		self.look_to_reg(BytecodeRegister::Base, stack_pos);
		self.compile(right, context)?;

		self.stack -= 1;

		self.result_push(op);

		self.load_from_reg(BytecodeRegister::Accum, stack_pos);

		Ok(())
	}

	pub fn build(self) -> Result<Vec<u8>, BytecodeError> {
		println!("Bytecode before optimizer:");

		let mut i = 0;

		for byte in &self.code_seg {
			println!("{i:02}: {byte}");

			i += byte.len();
		}

		let result = self.code_seg;
		// result.push(Bytecode::PushNumber((i + 1 + offset + 0x80) as VMWord));

		let mut asm_optimizer = AssemblerOptimizer::new(&result);

		let mut optimized = asm_optimizer.optimize();

		println!("Bytecode after optimizer:");

		let mut i = 0;

		for byte in &optimized {
			println!("{i:02}: {byte}");

			i += byte.len();
		}

		optimized.push(Bytecode::Halt);

		let bytes = Bytecode::asm(optimized)?;

		let mut binary = Vec::new();

		binary.extend(bytes);
		binary.extend(self.data_seg);

		Ok(binary)
	}

	fn compile(&mut self, ast: &Spanned<ASTNode>, context: &mut CompilerContext) -> Result<(), Spanned<CompilerError>> {
		println!("context = {context:?}");

		println!("stack_pos = {}; ast = {ast}", self.stack);

		let span = ast.span;

		match &ast.val {
			ASTNode::FunctionDefinition { name: Some(name), args, body, is_global } => {
				let jmp = self.pc();
				self.result_push(Bytecode::Jmp(0));

				if *is_global {
					self.global_functions.insert(name.val.clone(), self.pc());
				} else {
					context.insert_function(name.val.clone(), self.pc());
				}

				self.result_push(
					Bytecode::PushReg(
						BytecodeRegister::BasePointer
					)
				);
				self.result_push(
					Bytecode::MovRegToReg(
						BytecodeRegister::BasePointer,
						BytecodeRegister::StackPointer,
					)
				);

				let mut body_context = CompilerContext::new(Some(context));

				for (i, arg) in args.iter().enumerate().rev() {
					self.stack += 1;

					body_context.insert_var(&arg.val, i as VMWord);
				}

				self.compile(body, &mut body_context)?;

				self.result_push(
					Bytecode::PopReg(
						BytecodeRegister::BasePointer
					)
				);

				self.result_push(Bytecode::Ret);

				*self.code_seg.get_mut(jmp as usize).expect("result.len() < jmp") = Bytecode::Jmp(self.pc());
			},
			ASTNode::Return(value) => {
				self.compile(value, context)?;

				self.result_push(Bytecode::Ret);
			},
			ASTNode::Break(value) => {
				self.compile(value, context)?;

				let pc = self.pc();

				let Some(pos) = self.break_pos.last_mut() else {
					return Err(
						Spanned::from_with_span(CompilerError::BreakOutsideOfLoop, span)
					)
				};

				*pos = Some(pc);

				self.result_push(Bytecode::Jmp(0));
			},
			ASTNode::FunctionDefinition {..} => todo!("{}", ast.val),
			ASTNode::None => return Ok(()),
			&ASTNode::Boolean(x) => {
				self.stack += 1;

				self.result_push(Bytecode::PushNumber(x.into()));
			},
			&ASTNode::Number(x) => {
				self.stack += 1;

				self.result_push(Bytecode::PushNumber(x.cast_unsigned()));
			},
			&ASTNode::Float(x) => {
				self.stack += 1;

				self.result_push(Bytecode::PushNumber(x as VMWord));
			},
			ASTNode::String(x) => {
				self.stack += 1;

				self.result_push(Bytecode::PushRef(self.data_seg.len() as u32));

				self.data_seg.extend(x.clone().into_bytes());
			},
			ASTNode::List(values) => if !values.is_empty() {
				todo!();

				// self.call_func(
				// 	&Spanned::from_with_span("list", span),
				// 	values.iter(), context
				// )?;
			},
			ASTNode::Binary { left, op: wmatch!(BinaryOp::Range), right } => {
				self.call_func(
					&Spanned::from_with_span("range", left.span),
					[&**left, &**right].into_iter(), context
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
					self.compile(right, context)?;

					self.load_from_reg(BytecodeRegister::BasePointer, stack_pos);
				} else {
					let stack_pos = self.stack;

					self.compile(right, context)?;

					context.insert_var(name.to_owned(), stack_pos);
				}
			},
			ASTNode::Binary { left, op: wmatch!(BinaryOp::PlusAssignment), right } => {
				self.compile_assignment(left, right, Bytecode::Add, context)?;
			},
			ASTNode::Binary { left, op: wmatch!(BinaryOp::MinusAssignment), right } => {
				self.compile_assignment(left, right, Bytecode::Sub, context)?;
			},
			ASTNode::Binary { left, op: wmatch!(BinaryOp::MultiplyAssignment), right } => {
				self.compile_assignment(left, right, Bytecode::Mul, context)?;
			},
			ASTNode::Binary { left, op: wmatch!(BinaryOp::PowAssignment), right } => {
				self.compile_assignment(left, right, Bytecode::Pow, context)?;
			},
			ASTNode::Binary { left, op: wmatch!(BinaryOp::DivideAssignment), right } => {
				self.compile_assignment(left, right, Bytecode::Div, context)?;
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

				self.look_to_reg(BytecodeRegister::Accum, stack_pos);

				self.result_push(Bytecode::PushReg(BytecodeRegister::Accum));
			},
			ASTNode::Unary { op, value } => {
				self.compile(value, context)?;

				self.result_push(Bytecode::PopReg(BytecodeRegister::Accum));
				self.stack -= 1;

				self.result_push(Bytecode::from_unary_op(op));

				self.result_push(Bytecode::PushReg(BytecodeRegister::Accum));
				self.stack += 1;
			},
			ASTNode::Binary { left, op: wmatch!(BinaryOp::LessOrEquals), right } => {
				self.compile(left, context)?;
				self.compile(right, context)?;

				self.result_push(Bytecode::PopReg(BytecodeRegister::Base));
				self.stack -= 1;
				self.result_push(Bytecode::PopReg(BytecodeRegister::Accum));
				self.stack -= 1;

				self.result_push(Bytecode::from_binary_op(&BinaryOp::Less));
				self.result_push(Bytecode::from_unary_op(&UnaryOp::Not));

				self.result_push(Bytecode::PushReg(BytecodeRegister::Accum));
				self.stack += 1;
			},
			ASTNode::Binary { left, op: wmatch!(BinaryOp::Great), right } => {
				self.compile(right, context)?;
				self.compile(left, context)?;

				self.result_push(Bytecode::PopReg(BytecodeRegister::Base));
				self.stack -= 1;
				self.result_push(Bytecode::PopReg(BytecodeRegister::Accum));
				self.stack -= 1;

				self.result_push(Bytecode::from_binary_op(&BinaryOp::Less));

				self.result_push(Bytecode::PushReg(BytecodeRegister::Accum));
				self.stack += 1;
			},
			ASTNode::Binary { left, op, right } => {
				self.compile(left, context)?;
				self.compile(right, context)?;

				self.result_push(Bytecode::PopReg(BytecodeRegister::Base));
				self.stack -= 1;
				self.result_push(Bytecode::PopReg(BytecodeRegister::Accum));
				self.stack -= 1;

				self.result_push(Bytecode::from_binary_op(op));

				self.result_push(Bytecode::PushReg(BytecodeRegister::Accum));
				self.stack += 1;
			},
			ASTNode::Function { name, args } => {
				self.call_func(
					&Spanned::from_with_span(name, span),
					args.iter(), context
				)?;
			},
			ASTNode::While { condition, body, else_body: Some(else_body) } => {
				let cond = self.pc();

				self.compile(condition, context)?;

				let jif = self.pc();
				self.result_push(Bytecode::Jif(0));

				self.break_pos.push(None);

				self.compile(body, context)?;

				self.result_push(Bytecode::Jmp(cond));

				*self.code_seg.get_mut(jif as usize).expect("result.len() < jif") = Bytecode::Jif(self.pc());

				self.compile(else_body, context)?;

				if let Some(break_pos) = self.break_pos.pop().expect("Unexpected pop from self.break_pos") {
					*self.code_seg.get_mut(break_pos as usize).expect("result.len() < jmp") = Bytecode::Jmp(self.pc());
				}
			},
			ASTNode::While { condition, body, else_body: None } => {
				let cond = self.pc();

				self.compile(condition, context)?;

				let jif = self.pc();
				self.result_push(Bytecode::Jif(0));
				self.stack -= 1;

				self.break_pos.push(None);

				self.compile(body, context)?;

				self.result_push(Bytecode::Jmp(cond));

				*self.code_seg.get_mut(jif as usize).expect("result.len() < jif") = Bytecode::Jif(self.pc());

				if let Some(break_pos) = self.break_pos.pop().expect("Unexpected pop from self.break_pos") {
					*self.code_seg.get_mut(break_pos as usize).expect("result.len() < jmp") = Bytecode::Jmp(self.pc());
				}
			},
			ASTNode::If { condition, body, else_body: Some(else_body) } => {
				self.compile(condition, context)?;

				self.stack -= 1;
				let jif = self.pc();
				self.result_push(Bytecode::Jif(0));

				self.compile(body, context)?;
				let jmp = self.pc();
				self.result_push(Bytecode::Jmp(0));

				*self.code_seg.get_mut(jif as usize).expect("result.len() < jif") = Bytecode::Jif(self.pc());

				self.compile(else_body, context)?;

				*self.code_seg.get_mut(jmp as usize).expect("result.len() < jif") = Bytecode::Jmp(self.pc());
			},
			ASTNode::If { condition, body, else_body: None } => {
				self.compile(condition, context)?;

				self.stack -= 1;
				let jif = self.pc();

				self.result_push(Bytecode::Jif(0));

				self.compile(body, context)?;

				*self.code_seg.get_mut(jif as usize).expect("result.len() < jif") = Bytecode::Jif(self.pc());
			},
			ASTNode::Block(statements) => {
				let mut local_context = CompilerContext::new(Some(context));

				for statement in statements {
					self.compile(statement, &mut local_context)?;
				}
			},
		}

		Ok(())
	}

	pub fn compile_loop(&mut self, ast: &Spanned<ASTNode>) -> Result<(), Spanned<CompilerError>> {
		let mut context = CompilerContext::new(None);

		self.compile(ast, &mut context)?;

		// self.call_func(
		// 	&Spanned::from_with_span("main", 0),
		// 	[].iter(),
		// 	result,
		// 	&mut context
		// )?;

		Ok(())
	}
}
