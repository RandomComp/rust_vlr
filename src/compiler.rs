pub mod lexer;
pub mod parser;
pub mod optimizer;
pub mod native_func;

use crate::assembler::{BytecodeRegister, optimizer::AssemblerOptimizer};
use crate::compiler::native_func::{NativeFunctionArgsNumber, NativeFunctionID};
use crate::vm::{VMSWord, VMWord};
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
	LeftExprShouldBeId(ASTNode),
	#[error("Value of reference unary operation should be variable, not '{0}'")]
	ValueShouldBeVariable(ASTNode),
	#[error("Break outside of loop")]
	BreakOutsideOfLoop,
	#[error("Return outside of function")]
	ReturnOutsideOfFunction,
}

#[derive(Default, Debug, Clone, Copy)]
enum RegisterState {
	#[default]
	/// If register not affected
	NotAffected,
	/// If register affected while executing
	Affected,
	/// Result registr
	Result,
}

#[derive(Default, Copy, Clone, Debug)]
struct CompilerState {
	acc: RegisterState,
	base: RegisterState, cnt: RegisterState,
}

#[derive(Debug)]
struct CompilerContext<'a> {
	functions: HashMap<String, usize>,

	vars: HashMap<String, i32>,
	parent: Option<&'a CompilerContext<'a>>,
}

impl<'a> CompilerContext<'a> {
	fn new(parent: Option<&'a Self>) -> Self {
		Self {
			functions: HashMap::new(),
			vars: HashMap::new(),
			parent
		}
	}

	fn get_var(&self, name: &str) -> Option<i32> {
		if let Some(&value) = self.vars.get(name) {
			Some(value)
		} else if let Some(parent) = self.parent {
			parent.get_var(name)
		} else {
			None
		}
	}
	fn get_function(&self, name: &str) -> Option<usize> {
		if let Some(&value) = self.functions.get(name) {
			Some(value)
		} else if let Some(parent) = self.parent {
			parent.get_function(name)
		} else {
			None
		}
	}

	fn insert_var<T>(&mut self, name: T, stack_pos: i32) where T: Into<String> {
		self.vars.insert(name.into(), stack_pos);
	}
	fn insert_function<T>(&mut self, name: T, label: usize) where T: Into<String> {
		self.functions.insert(name.into(), label);
	}
}

pub struct Segments {
	/// Code segment
	pub code: Vec<Bytecode>,
	/// Read-only data segment
	pub data: Vec<u8>,
}

impl Segments {
	pub fn new() -> Self {
		Self {
			code: Vec::new(),
			data: Vec::new(),
		}
	}
}

pub struct Compiler {
	labels_id: usize,

	stack: u32,
	break_pos: Vec<Option<u32>>,
	global_functions: HashMap<String, usize>,

	/// Read-only data segment
	pub data: Vec<u8>,
	pc: usize,
}

impl Compiler {
	pub fn new() -> Self {
		Self {
			labels_id: 0,
			stack: 0,
			break_pos: Vec::new(),
			global_functions: HashMap::new(),

			data: Vec::new(),
			pc: 0,
		}
	}

	fn result_push(&mut self, buf: &mut Vec<Bytecode>, code: Bytecode) {
		self.pc += code.len();

		buf.push(code);
	}

	fn look_to_reg(&mut self, buf: &mut Vec<Bytecode>, reg: BytecodeRegister, stack_pos: VMSWord) {
		self.result_push(buf, Bytecode::LoadToReg(reg, BytecodeRegister::StackPointer, stack_pos));

		self.stack += 1;
	}

	fn load_from_reg(&mut self, buf: &mut Vec<Bytecode>, reg: BytecodeRegister, stack_pos: VMSWord) {
		self.result_push(buf, Bytecode::LoadFromReg(reg, BytecodeRegister::StackPointer, stack_pos));

		self.stack -= 1;
	}

	fn save_state(&mut self, buf: &mut Vec<Bytecode>, state: CompilerState) {
		if let RegisterState::Affected = state.acc {
			self.result_push(buf, Bytecode::PushReg(BytecodeRegister::Accum));
		}
		if let RegisterState::Affected = state.base {
			self.result_push(buf, Bytecode::PushReg(BytecodeRegister::Base));
		}
		if let RegisterState::Affected = state.cnt {
			self.result_push(buf, Bytecode::PushReg(BytecodeRegister::Counter));
		}
	}
	fn load_state(&mut self, buf: &mut Vec<Bytecode>, state: CompilerState) {
		if let RegisterState::Affected = state.cnt {
			self.result_push(buf, Bytecode::PopReg(BytecodeRegister::Counter));
		}
		if let RegisterState::Affected = state.base {
			self.result_push(buf, Bytecode::PopReg(BytecodeRegister::Base));
		}
		if let RegisterState::Affected = state.acc {
			self.result_push(buf, Bytecode::PopReg(BytecodeRegister::Accum));
		}
	}

	fn get_result_state(state: CompilerState) -> BytecodeRegister {
		let acc_is_res = matches!(state.acc, RegisterState::Result);
		let base_is_res = matches!(state.base, RegisterState::Result);
		let cnt_is_res = matches!(state.cnt, RegisterState::Result);
		let is_multiple_res =
			(base_is_res && cnt_is_res) ||
			(acc_is_res && (base_is_res || cnt_is_res));

		if is_multiple_res {
			unreachable!("result registers cannot be multiple: {state:?}")
		} else if acc_is_res {
			BytecodeRegister::Accum
		} else if base_is_res {
			BytecodeRegister::Base
		} else if cnt_is_res {
			BytecodeRegister::Counter
		} else {
			unreachable!("no such result: {state:?}")
		}
	}
	fn push_result_to_stack(&mut self, buf: &mut Vec<Bytecode>, state: CompilerState) {
		let reg = Self::get_result_state(state);

		self.result_push(buf, Bytecode::PushReg(reg));
	}

	fn mov_result_to_reg(&mut self, buf: &mut Vec<Bytecode>, reg: BytecodeRegister, state: CompilerState) {
		let reg2 = Self::get_result_state(state);

		self.result_push(buf, Bytecode::MovRegToReg(reg, reg2));
	}

	fn call_func<'a, T>(&mut self, buf: &mut Vec<Bytecode>, name: &Spanned<&str>, args: T, context: &mut CompilerContext) -> Result<CompilerState, Spanned<CompilerError>>
	where T: Iterator<Item=&'a Spanned<ASTNode>> {
		if let Ok(id) = NativeFunctionID::try_from(name.val) {
			let mut len = 0;

			for arg in args {
				let (compiled_arg, state) = self.compile(arg, context)?;

				buf.extend(compiled_arg);

				self.push_result_to_stack(buf, state);

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
					self.result_push(buf, Bytecode::PushNumber(len)),
			}

			self.result_push(buf, Bytecode::NativeCall(id));
			self.stack -= len;
			self.stack += 1;

			return Ok(
				CompilerState {
					acc: RegisterState::Result,
					base: RegisterState::NotAffected,
					cnt: RegisterState::NotAffected
				}
			)
		} else if let Some(&pos) = self.global_functions.get(name.val) {
			for arg in args {
				let (compiled_arg, state) = self.compile(arg, context)?;

				buf.extend(compiled_arg);

				self.push_result_to_stack(buf, state);
			}

			self.result_push(buf, Bytecode::CallLabel(pos));

			return Ok(
				CompilerState {
					acc: RegisterState::Result,
					base: RegisterState::NotAffected,
					cnt: RegisterState::NotAffected
				}
			)
		} else if let Some(pos) = context.get_function(name.val) {
			for arg in args {
				let (compiled_arg, state) = self.compile(arg, context)?;

				buf.extend(compiled_arg);

				self.push_result_to_stack(buf, state);
			}

			self.result_push(buf, Bytecode::CallLabel(pos));

			return Ok(
				CompilerState {
					acc: RegisterState::Result,
					base: RegisterState::NotAffected,
					cnt: RegisterState::NotAffected
				}
			)
		}

		Err(
			Spanned::from_with_span(
				CompilerError::UnknownFunctionCall(name.val.to_owned()),
				name.span
			)
		)
	}

	fn compile_assignment(&mut self, buf: &mut Vec<Bytecode>, left: &Spanned<ASTNode>, right: &Spanned<ASTNode>, op: &BinaryOp, context: &mut CompilerContext) -> Result<(), Spanned<CompilerError>> {
		let ASTNode::Variable(name) = &left.val else {
			return Err(
				Spanned::from_with_span(
					CompilerError::LeftExprShouldBeId(left.val.clone()),
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

		self.look_to_reg(buf, BytecodeRegister::Base, stack_pos);
		self.compile(right, context)?;

		self.stack -= 1;

		match op {
			BinaryOp::PlusAssignment => {
				self.result_push(buf, Bytecode::Add(BytecodeRegister::Accum, BytecodeRegister::Base));
			}
			BinaryOp::MinusAssignment => {
				self.result_push(buf, Bytecode::Sub(BytecodeRegister::Accum, BytecodeRegister::Base));
			}
			BinaryOp::MultiplyAssignment => {
				self.result_push(buf, Bytecode::Mul(BytecodeRegister::Accum, BytecodeRegister::Base));
			}
			BinaryOp::DivideAssignment => {
				self.result_push(buf, Bytecode::Div(BytecodeRegister::Accum, BytecodeRegister::Base));
			}
			// BinaryOp::RemainderAssignment => {
			// 	self.result_push(Bytecode::Re(BytecodeRegister::Accum, BytecodeRegister::Base));
			// }
			BinaryOp::PowAssignment => {
				self.result_push(buf, Bytecode::Pow(BytecodeRegister::Accum, BytecodeRegister::Base));
			}
			_ => unreachable!("{op} is not valid assignment binary operator or not handled")
		}

		self.load_from_reg(buf, BytecodeRegister::Accum, stack_pos);

		Ok(())
	}

	pub fn build(mut self, code: &Spanned<ASTNode>) -> Result<Segments, Spanned<CompilerError>> {
		println!("Bytecode before optimizer:");

		let mut i = 0;

		let mut code = self.compile_loop(code)?;

		for byte in &code {
			println!("{i:02}: {byte}");

			i += byte.len();
		}

		// result.push(Bytecode::PushNumber((i + 1 + offset + 0x80) as VMWord));

// 		let mut asm_optimizer = AssemblerOptimizer::new(&result);
//
// 		let mut optimized = asm_optimizer.optimize();

// 		println!("Bytecode after optimizer:");
//
// 		let mut i = 0;
//
// 		for byte in &optimized {
// 			println!("{i:02}: {byte}");
//
// 			i += byte.len();
// 		}

		code.push(Bytecode::Halt);

		Ok(
			Segments { code, data: self.data }
		)
	}

	fn insert_global_function<T>(&mut self, name: T) -> usize where T: Into<String> {
		let label = self.labels_id;
		self.labels_id += 1;

		self.global_functions.insert(name.into(), label);

		label
	}

	fn insert_function<T>(&mut self, name: T, context: &mut CompilerContext) -> usize where T: Into<String> {
		let label = self.labels_id;
		self.labels_id += 1;

		context.insert_function(name.into(), label);

		label
	}

	fn compile(&mut self, ast: &Spanned<ASTNode>, context: &mut CompilerContext) -> Result<(Vec<Bytecode>, CompilerState), Spanned<CompilerError>> {
		println!("context = {context:?}");

		println!("stack_pos = {}; ast = {ast}", self.stack);

		let span = ast.span;

		let mut buf = Vec::new();

		match &ast.val {
			ASTNode::FunctionDefinition { name: Some(name), args, body, is_global } => {
				self.result_push(&mut buf, Bytecode::Jmp(self.pc as u32));

				if *is_global {
					self.insert_global_function(name.val.clone());
				} else {
					self.insert_function(name.val.clone(), context);
				}

				self.result_push(&mut buf,
					Bytecode::PushReg(
						BytecodeRegister::BasePointer
					)
				);
				self.result_push(&mut buf,
					Bytecode::MovRegToReg(
						BytecodeRegister::BasePointer,
						BytecodeRegister::StackPointer,
					)
				);

				let mut body_context = CompilerContext::new(Some(context));

				for (i, arg) in args.iter().enumerate().rev() {
					self.stack += 1;

					body_context.insert_var(&arg.val, i as VMSWord);
				}

				let (body_compiled, state) = self.compile(body, &mut body_context)?;

				buf.extend(body_compiled);
				self.result_push(&mut buf,
					Bytecode::PopReg(
						BytecodeRegister::BasePointer
					)
				);
				self.result_push(&mut buf, Bytecode::Ret);

				// CompilerState::default()
			},
			ASTNode::Return(value) => {
				buf.extend(self.compile(value, context)?.0);
				self.result_push(&mut buf,
					Bytecode::PopReg(
						BytecodeRegister::Accum
					)
				);

				self.result_push(&mut buf, Bytecode::Ret);

				// CompilerState::default()
			},
			ASTNode::Break(value) => {
				buf.extend(self.compile(value, context)?.0);

				let pc = self.pc;

				let Some(pos) = self.break_pos.last_mut() else {
					return Err(
						Spanned::from_with_span(CompilerError::BreakOutsideOfLoop, span)
					)
				};

				*pos = Some(pc as u32);

				self.result_push(&mut buf, Bytecode::Jmp(0));
			},
			ASTNode::FunctionDefinition {..} => todo!("{}", ast.val),
			ASTNode::None => {},
			&ASTNode::Boolean(x) => {
				self.stack += 1;

				self.result_push(&mut buf, Bytecode::PushNumber(x.into()));
			},
			&ASTNode::Number(x) => {
				self.stack += 1;

				self.result_push(&mut buf, Bytecode::PushNumber(x.cast_unsigned()));
			},
			&ASTNode::Float(x) => {
				self.stack += 1;

				self.result_push(&mut buf, Bytecode::PushNumber(x as VMWord));
			},
			ASTNode::String(x) => {
				self.stack += 1;

				self.result_push(&mut buf, Bytecode::PushRef(self.data.len() as u32));

				self.data.extend(x.clone().into_bytes());
			},
			ASTNode::List(values) => if !values.is_empty() {
				todo!();

				// self.call_func(
				// 	&Spanned::from_with_span("list", span),
				// 	values.iter(), context
				// )?;
			},
			ASTNode::Binary { left, op: wmatch!(BinaryOp::Range), right } => {
				self.call_func(&mut buf,
					&Spanned::from_with_span("range", left.span),
					[&**left, &**right].into_iter(), context
				)?;
			},
			ASTNode::Binary { left, op: wmatch!(BinaryOp::Assignment), right } => {
				let ASTNode::Variable(name) = &left.val else {
					return Err(
						Spanned::from_with_span(
							CompilerError::LeftExprShouldBeId(left.val.clone()),
							span
						)
					);
				};

				if let Some(stack_pos) = context.get_var(name) {
					self.compile(right, context)?;

					self.load_from_reg(&mut buf, BytecodeRegister::BasePointer, stack_pos);
				} else {
					// let stack_pos = self.stack;

					self.compile(right, context)?;

					// todo: читать CompilerState

					// context.insert_var(name.to_owned(), stack_pos);
				}
			},
			ASTNode::Binary {
				left,
				op:
					op @ wmatch!(
						BinaryOp::PlusAssignment |
						BinaryOp::MinusAssignment |
						BinaryOp::MultiplyAssignment |
						BinaryOp::DivideAssignment |
						BinaryOp::RemainderAssignment |
						BinaryOp::PowAssignment
					),
				right
			} => {
				self.compile_assignment(&mut buf, left, right, &op.val, context)?;
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

				self.look_to_reg(&mut buf, BytecodeRegister::Accum, stack_pos);

				self.result_push(&mut buf, Bytecode::PushReg(BytecodeRegister::Accum));
			},
			ASTNode::Unary { op: UnaryOp::Ref, value } => {
				let ASTNode::Variable(name) = &value.val else {
					return Err(
						Spanned::from_with_span(
							CompilerError::ValueShouldBeVariable(value.val.clone()),
							span
						)
					);
				};

				let Some(stack_pos) = context.get_var(name) else {
					return Err(
						Spanned::from_with_span(
							CompilerError::NotKnownAtThisScope(name.clone()),
							span
						)
					);
				};

				self.result_push(&mut buf, Bytecode::MovRegToReg(BytecodeRegister::Accum, BytecodeRegister::BasePointer));
				self.result_push(&mut buf, Bytecode::MovU32ToReg(BytecodeRegister::Base, (stack_pos + 4).cast_unsigned()));
				self.result_push(&mut buf, Bytecode::Sub(BytecodeRegister::Accum, BytecodeRegister::Base));

				self.stack += 1;

				// CompilerState {
				// 	acc: RegisterState::Result,
				// 	base: RegisterState::Affected,
				// 	cnt: RegisterState::NotAffected,
				// }
			},
			ASTNode::Unary { op, value } => {
				self.compile(value, context)?;

				self.result_push(&mut buf, Bytecode::PopReg(BytecodeRegister::Accum));
				self.stack -= 1;

				self.result_push(&mut buf, Bytecode::from_unary_op(op, BytecodeRegister::Accum));

				self.result_push(&mut buf, Bytecode::PushReg(BytecodeRegister::Accum));
				self.stack += 1;
			},
			ASTNode::Binary { left, op: wmatch!(BinaryOp::LessOrEquals), right } => {
				self.compile(left, context)?;
				self.compile(right, context)?;

				self.result_push(&mut buf, Bytecode::PopReg(BytecodeRegister::Base));
				self.stack -= 1;
				self.result_push(&mut buf, Bytecode::PopReg(BytecodeRegister::Accum));
				self.stack -= 1;

				self.result_push(&mut buf, Bytecode::Lt(BytecodeRegister::Accum, BytecodeRegister::Base));
				self.result_push(&mut buf, Bytecode::Not(BytecodeRegister::Accum));

				self.result_push(&mut buf, Bytecode::PushReg(BytecodeRegister::Accum));
				self.stack += 1;
			},
			ASTNode::Binary { left, op: wmatch!(BinaryOp::Neq), right } => {
				self.compile(left, context)?;
				self.compile(right, context)?;

				self.result_push(&mut buf, Bytecode::PopReg(BytecodeRegister::Base));
				self.stack -= 1;
				self.result_push(&mut buf, Bytecode::PopReg(BytecodeRegister::Accum));
				self.stack -= 1;

				self.result_push(&mut buf, Bytecode::Eq(BytecodeRegister::Accum, BytecodeRegister::Base));
				self.result_push(&mut buf, Bytecode::Not(BytecodeRegister::Accum));

				self.result_push(&mut buf, Bytecode::PushReg(BytecodeRegister::Accum));
				self.stack += 1;
			},
			ASTNode::Binary { left, op: wmatch!(BinaryOp::Great), right } => {
				self.compile(right, context)?;
				self.compile(left, context)?;

				self.result_push(&mut buf, Bytecode::PopReg(BytecodeRegister::Base));
				self.stack -= 1;
				self.result_push(&mut buf, Bytecode::PopReg(BytecodeRegister::Accum));
				self.stack -= 1;

				self.result_push(&mut buf, Bytecode::Lt(BytecodeRegister::Accum, BytecodeRegister::Base));

				self.result_push(&mut buf, Bytecode::PushReg(BytecodeRegister::Accum));
				self.stack += 1;
			},
			ASTNode::Binary { left, op, right } => {
				let (left_compiled, left_state) = self.compile(left, context)?;
				let (right_compiled, right_state) = self.compile(right, context)?;

				buf.extend(left_compiled);
				self.save_state(&mut buf, right_state);

				self.mov_result_to_reg(&mut buf, BytecodeRegister::Accum, left_state);

				buf.extend(right_compiled);
				self.load_state(&mut buf, right_state);

				self.mov_result_to_reg(&mut buf, BytecodeRegister::Base, right_state);

				self.result_push(&mut buf, Bytecode::from_binary_op(op, BytecodeRegister::Accum, BytecodeRegister::Base));

				// Compile
			},
			ASTNode::Function { name, args } => {
				self.call_func(&mut buf,
					&Spanned::from_with_span(name, span),
					args.iter(), context
				)?;
			},
			ASTNode::While { condition, body, else_body: Some(else_body) } => {
				todo!();
// 				let cond = self.pc;
//
// 				self.compile(condition, context)?;
//
// 				let jif = self.pc;
// 				self.result_push(&mut buf, Bytecode::Jif(0));
//
// 				self.break_pos.push(None);
//
// 				self.compile(body, context)?;
//
// 				self.result_push(&mut buf, Bytecode::Jmp(cond as u32));
//
// 				*self.segs.code.get_mut(jif as usize).expect("result.len() < jif") = Bytecode::Jif(self.pc as u32);
//
// 				self.compile(else_body, context)?;
//
// 				if let Some(break_pos) = self.break_pos.pop().expect("Unexpected pop from self.break_pos") {
// 					*self.segs.code.get_mut(break_pos as usize).expect("result.len() < jmp") = Bytecode::Jmp(self.pc as u32);
// 				}
			},
			ASTNode::While { condition, body, else_body: None } => {
				todo!();
// 				let cond = self.pc;
//
// 				buf.extend(self.compile(condition, context)?);
//
// 				let jif = self.pc;
// 				self.result_push(&mut buf, Bytecode::Jif(0));
// 				self.stack -= 1;
//
// 				self.break_pos.push(None);
//
// 				self.compile(body, context)?;
//
// 				self.result_push(&mut buf, Bytecode::Jmp(cond as u32));
//
// 				*self.segs.code.get_mut(jif as usize).expect("result.len() < jif") = Bytecode::Jif(self.pc as u32);
//
// 				if let Some(break_pos) = self.break_pos.pop().expect("Unexpected pop from self.break_pos") {
// 					*self.segs.code.get_mut(break_pos as usize).expect("result.len() < jmp") = Bytecode::Jmp(self.pc as u32);
// 				}
			},
			ASTNode::If { condition, body, else_body: Some(else_body) } => {
				todo!();
// 				self.compile(condition, context)?;
//
// 				self.stack -= 1;
// 				let jif = self.pc;
// 				self.result_push(&mut buf, Bytecode::Jif(0));
//
// 				self.compile(body, context)?;
// 				let jmp = self.pc;
// 				self.result_push(&mut buf, Bytecode::Jmp(0));
//
// 				*self.segs.code.get_mut(jif as usize).expect("result.len() < jif") = Bytecode::Jif(self.pc as u32);
//
// 				self.compile(else_body, context)?;
//
// 				*self.segs.code.get_mut(jmp as usize).expect("result.len() < jif") = Bytecode::Jmp(self.pc as u32);
			},
			ASTNode::If { condition, body, else_body: None } => {
				todo!();
// 				self.compile(condition, context)?;
//
// 				self.stack -= 1;
// 				let jif = self.pc;
//
// 				self.result_push(Bytecode::Jif(0));
//
// 				self.compile(body, context)?;
//
// 				*self.segs.code.get_mut(jif as usize).expect("result.len() < jif") = Bytecode::Jif(self.pc as u32);
			},
			ASTNode::Block(statements) => {
				let mut local_context = CompilerContext::new(Some(context));

				for statement in statements {
					self.compile(statement, &mut local_context)?;
				}
			},
		}

		Ok((buf, CompilerState::default()))
	}

	pub fn compile_loop(&mut self, ast: &Spanned<ASTNode>) -> Result<Vec<Bytecode>, Spanned<CompilerError>> {
		let mut context = CompilerContext::new(None);

		let result = self.compile(ast, &mut context)?.0;

		// self.call_func(
		// 	&Spanned::from_with_span("main", 0),
		// 	[].iter(),
		// 	result,
		// 	&mut context
		// )?;

		Ok(result)
	}
}
