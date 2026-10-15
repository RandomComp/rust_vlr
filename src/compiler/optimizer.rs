use std::{collections::HashMap, mem};

use crate::compiler::parser::{BinaryOp, ASTNode, UnaryOp};
use crate::uni_type::{UniResult, UniResultError};
use crate::wmatch;
use crate::wrapper::Spanned;

#[derive(thiserror::Error, Clone, PartialEq, Debug)]
pub enum OptimizerError {
	#[error("{0}")]
	RawUniResult(#[from] UniResultError),
	// #[error("'{0}' variable is not known in current scope")]
	// NotKnownAtThisScope(String),
}

pub struct Optimizer {
	pub unused_vars: Vec<String>,
	// constants: HashMap<String, UniResult>,
}

type SpannedASTNode = Spanned<ASTNode>;
type SpannedUniResult = Spanned<UniResult>;

impl Optimizer {
	pub fn new() -> Self {
		Self {unused_vars: Vec::new()} // constants: HashMap::new()}
	}

	pub fn have_no_effects(ast: &ASTNode) -> bool {
		match ast {
			ASTNode::Function { .. } |
			ASTNode::FunctionDefinition { name: Some(_), .. } |
			ASTNode::Binary {
				left: _,
				op: wmatch!(
					BinaryOp::Assignment |
					BinaryOp::PlusAssignment |
					BinaryOp::MinusAssignment |
					BinaryOp::MultiplyAssignment |
					BinaryOp::PowAssignment |
					BinaryOp::DivideAssignment
				),
				right: _
			} => false,
			ASTNode::Binary { left, op: wmatch!(BinaryOp::LogicalAnd | BinaryOp::LogicalOr), right } => {
				Self::have_no_effects(left) && Self::have_no_effects(right)
			},
			_ => true,
		}
	}

	pub fn simplify(ast: &mut Spanned<ASTNode>) {
		let span = ast.span;

		match &mut ast.val {
			ASTNode::Binary {
				left,
				op: wmatch!(BinaryOp::Multiply),
				right
			} if left.val == ASTNode::Float(0.0) || right.val == ASTNode::Float(0.0) => {
				ast.val = ASTNode::Float(0.0);
			},
			ASTNode::Binary {
				left,
				op: wmatch!(BinaryOp::Multiply),
				right
			} if right.val == ASTNode::Float(1.0) => {
				 *ast = mem::take(left);
			},
			ASTNode::Binary {
				left,
				op: wmatch!(BinaryOp::Multiply),
				right
			} if left.val == ASTNode::Float(1.0) => {
				 *ast = mem::take(right);
			},
			ASTNode::Binary {
				left,
				op: wmatch!(BinaryOp::Plus),
				right
			} if let ASTNode::Float(0.0) = left.val => {
				 *ast = mem::take(right);
			},
			ASTNode::Binary {
				left,
				op: wmatch!(BinaryOp::Minus),
				right
			} if let ASTNode::Float(0.0) = left.val => {
				ast.val = ASTNode::Unary {
					op: UnaryOp::Minus, value: mem::take(right)
				}
			},
			ASTNode::Binary {
				left,
				op: wmatch!(BinaryOp::Plus | BinaryOp::Minus),
				right
			} if let ASTNode::Float(0.0) = right.val => {
				 *ast = mem::take(left);
			},
			ASTNode::Binary {
				left,
				op: op_token @ wmatch!(BinaryOp::Plus),
				right
			} if left == right => {
				ast.val = ASTNode::Binary {
					left: mem::take(left),
					op: Spanned::from_with_span(BinaryOp::Multiply, op_token.span),
					right: Box::new(
						Spanned::from_with_span(ASTNode::Float(2.0), right.span)
					),
				}
			},
			ASTNode::Binary {
				left,
				op: wmatch!(BinaryOp::Minus),
				right
			} if left == right => {
				ast.val = ASTNode::Float(0.0);
			},
			ASTNode::Binary {
				left,
				op: wmatch!(BinaryOp::Equals | BinaryOp::LessOrEquals | BinaryOp::GreatOrEquals),
				right
			} if left == right => {
				ast.val = ASTNode::Boolean(true);
			},
			ASTNode::Binary {
				left,
				op: wmatch!(BinaryOp::Neq | BinaryOp::Less | BinaryOp::Great),
				right
			} if left == right => {
				ast.val = ASTNode::Boolean(false);
			},
			ASTNode::Binary {
				left,
				op: wmatch!(BinaryOp::LogicalAnd),
				right
			} => {
				match (&left.val, &right.val) {
					(ASTNode::Boolean(false), _) => {
						ast.val = ASTNode::Boolean(false);
					},
					(_, ASTNode::Boolean(false)) if Self::have_no_effects(left) => {
						ast.val = ASTNode::Boolean(false);
					},
					(_, ASTNode::Boolean(true)) => {
						*ast = mem::take(left);
					},
					(ASTNode::Boolean(true), _) => {
						*ast = mem::take(right);
					},
					_ => {},
				}
			},
			ASTNode::Binary {
				left,
				op: wmatch!(BinaryOp::LogicalOr),
				right
			} => {
				match (&left.val, &right.val) {
					(ASTNode::Boolean(true), _) |
					(_, ASTNode::Boolean(true)) if Self::have_no_effects(left) && Self::have_no_effects(right) => {
						ast.val = ASTNode::Boolean(true);
					},
					(_, ASTNode::Boolean(false)) => {
						*ast = mem::take(left);
					},
					(ASTNode::Boolean(false), _) => {
						*ast = mem::take(right);
					},
					_ => {},
				}
			},
			_ => {},
		}
	}

	pub fn fold(&mut self, ast: &mut Spanned<ASTNode>) -> Result<(), Spanned<OptimizerError>> {
		let span = ast.span;

		Self::simplify(ast);

		match &mut ast.val {
			ASTNode::None | ASTNode::Boolean(..) |
			ASTNode::Number(..) | ASTNode::Float(..) |
			ASTNode::String(..) => {},
			ASTNode::Variable(name) => {
				if let Some(index) = self.unused_vars.iter().position(|v| v == name) {
					self.unused_vars.remove(index);
				}

				// let constant_value = self.constants.get(name).ok_or(OptimizerError::NotKnownAtThisScope(name.to_owned()))?;
				// ast.value = constant_value.clone().try_into()?;
			},
			ASTNode::Binary { left, op: wmatch!(BinaryOp::Assignment), right } if let ASTNode::Variable(name) = &left.val => {
				self.fold(right)?;

				self.unused_vars.push(name.to_owned());

				// let Ok(constant_value): Result<UniResult, _> = (**right).clone().try_into() else {
				// 	return Ok(())
				// };

				// self.constants.insert(name.to_owned(), constant_value);

				// ast.value = ASTNodeEnum::None;
			},
			ASTNode::Binary {
				left: _,
				op: wmatch!(
					BinaryOp::PlusAssignment |
					BinaryOp::MinusAssignment |
					BinaryOp::MultiplyAssignment |
					BinaryOp::PowAssignment |
					BinaryOp::DivideAssignment
				),
				right } => {
				self.fold(right)?;
			},
			ASTNode::Unary { op: UnaryOp::Minus, value } => {
				self.fold(value)?;

				let Ok(value): Result<Spanned<UniResult>, _> = (**value).clone().try_into() else {
					return Ok(())
				};

				*ast = UniResult::calc_unary_by_op(&value, &UnaryOp::Minus)?.try_into()?;
			},
			ASTNode::Unary { op: UnaryOp::Not, value } => {
				self.fold(value)?;

				let Ok(value): Result<Spanned<UniResult>, _> = (**value).clone().try_into() else {
					return Ok(())
				};

				*ast = UniResult::calc_unary_by_op(&value, &UnaryOp::Not)?.try_into()?;
			},
			ASTNode::Binary { left, op, right } => {
				let left_is_err = self.fold(left).is_err();
				let right_is_err = self.fold(right).is_err();

				if left_is_err || right_is_err {
					return Ok(())
				}

				let Ok(left) = SpannedUniResult::try_from((**left).clone()) else {
					return Ok(())
				};

				let Ok(right) = SpannedUniResult::try_from((**right).clone()) else {
					return Ok(())
				};

				*ast = match UniResult::calc_binary_by_op(&left, &right, op).map(|v| SpannedASTNode::try_from(v)).flatten() {
					Ok(x) => x,
					Err(
						Spanned { val: UniResultError::UnsupportedOperation(..), .. }
					) => return Ok(()),
					Err(e) => return Err(e.into()),
				};
			},
			ASTNode::List(values) => {
				for value in values {
					self.fold(value)?;
				}
			},
			ASTNode::Function { name: _, args } => {
				for arg in args {
					self.fold(arg)?;
				}
			},
			ASTNode::FunctionDefinition { name: _, args: _, body, is_global } => {
				self.fold(body)?;
			},
			ASTNode::If { condition, body, else_body: None } => {
				self.fold(body)?;

				self.fold(condition)?;

				let condition_folded: Result<Spanned<UniResult>, _> = (**condition).clone().try_into();

				if let Ok(condition) = condition_folded {
					if bool::try_from(condition)? {
						*ast = *body.clone();
					} else {
						ast.val = ASTNode::None;
					}
				}
			},
			ASTNode::If { condition, body, else_body: Some(else_body) } => {
				self.fold(condition)?;

				let condition_folded: Result<Spanned<UniResult>, _> = (**condition).clone().try_into();
				self.fold(body)?;
				self.fold(else_body)?;

				if let Ok(condition) = condition_folded {
					if bool::try_from(condition)? {
						*ast = *body.clone();
					} else {
						*ast = *else_body.clone();
					}
				}
			},
			ASTNode::While { condition, body, else_body: None } => {
				self.fold(body)?;

				self.fold(condition)?;

				let Ok(condition): Result<Spanned<UniResult>, _> = (**condition).clone().try_into() else {
					return Ok(())
				};

				if !bool::try_from(condition)? {
					ast.val = ASTNode::None;
				}
			},
			ASTNode::While { condition, body, else_body: Some(else_body) } => {
				self.fold(condition)?;
				self.fold(body)?;
				self.fold(else_body)?;

				let Ok(condition): Result<Spanned<UniResult>, _> = (**condition).clone().try_into() else {
					return Ok(())
				};

				if !bool::try_from(condition)? {
					ast.val = ASTNode::None;
				}
			},
			ASTNode::Break(value) |
			ASTNode::Return(value) => {
				self.fold(value)?;
			},
			ASTNode::Block(exprs) => {
				for expr in exprs {
					self.fold(expr)?;
				}
			},
		}

		Ok(())
	}
}
