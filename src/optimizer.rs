use std::{collections::HashMap, mem};

use crate::parser::{BinaryOp, RawASTNode, UnaryOp};
use crate::uni_type::{RawUniResult, RawUniResultError};
use crate::wrapper::{ASTNode, OptimizerError, UniResult, UniResultError};

#[derive(thiserror::Error, Clone, PartialEq, Debug)]
pub enum RawOptimizerError {
	#[error("{0}")]
	UniResultError(#[from] UniResultError),
	// #[error("'{0}' variable is not known in current scope")]
	// NotKnownAtThisScope(String),
}

pub struct Optimizer {
	pub unused_vars: Vec<String>,
	// constants: HashMap<String, UniResult>,
}

impl Optimizer {
	pub fn new() -> Self {
		Self {unused_vars: Vec::new()} // constants: HashMap::new()}
	}

	pub fn have_no_effects(ast: &ASTNode) -> bool {
		match &ast.val {
			RawASTNode::Function { .. } |
			RawASTNode::FunctionDefinition { name: Some(_), .. } |
			RawASTNode::Binary {
				left: _,
				op: BinaryOp::Assignment |
					BinaryOp::PlusAssignment |
					BinaryOp::MinusAssignment |
					BinaryOp::MultiplyAssignment |
					BinaryOp::PowAssignment |
					BinaryOp::DivideAssignment,
				right: _
			} => false,
			RawASTNode::Binary { left, op: BinaryOp::LogicalAnd | BinaryOp::LogicalOr, right } => {
				Self::have_no_effects(left) && Self::have_no_effects(right)
			},
			_ => true,
		}
	}

	pub fn simplify(ast: &mut ASTNode) {
		let span = ast.span;

		match &mut ast.val {
			RawASTNode::Binary {
				left,
				op: BinaryOp::Multiply,
				right
			} if left.val == RawASTNode::Float(0.0) || right.val == RawASTNode::Float(0.0) => {
				ast.val = RawASTNode::Float(0.0);
			},
			RawASTNode::Binary {
				left,
				op: BinaryOp::Multiply,
				right
			} if right.val == RawASTNode::Float(1.0) => {
				 *ast = mem::take(left);
			},
			RawASTNode::Binary {
				left,
				op: BinaryOp::Multiply,
				right
			} if left.val == RawASTNode::Float(1.0) => {
				 *ast = mem::take(right);
			},
			RawASTNode::Binary {
				left,
				op: BinaryOp::Plus,
				right
			} if let RawASTNode::Float(0.0) = left.val => {
				 *ast = mem::take(right);
			},
			RawASTNode::Binary {
				left,
				op: BinaryOp::Minus,
				right
			} if let RawASTNode::Float(0.0) = left.val => {
				ast.val = RawASTNode::Unary {
					op: UnaryOp::Minus, value: mem::take(right)
				}
			},
			RawASTNode::Binary {
				left,
				op: BinaryOp::Plus | BinaryOp::Minus,
				right
			} if let RawASTNode::Float(0.0) = right.val => {
				 *ast = mem::take(left);
			},
			RawASTNode::Binary {
				left,
				op: BinaryOp::Plus,
				right
			} if left == right => {
				ast.val = RawASTNode::Binary {
					left: mem::take(left),
					op: BinaryOp::Multiply,
					right: Box::new(ASTNode::from(RawASTNode::Float(2.0)).with_pos(right.span)),
				}
			},
			RawASTNode::Binary {
				left,
				op: BinaryOp::Minus,
				right
			} if left == right => {
				ast.val = RawASTNode::Float(0.0);
			},
			RawASTNode::Binary {
				left,
				op: BinaryOp::Equals | BinaryOp::LessOrEquals | BinaryOp::GreatOrEquals,
				right
			} if left == right => {
				ast.val = RawASTNode::Boolean(true);
			},
			RawASTNode::Binary {
				left,
				op: BinaryOp::Neq | BinaryOp::Less | BinaryOp::Great,
				right
			} if left == right => {
				ast.val = RawASTNode::Boolean(false);
			},
			RawASTNode::Binary {
				left,
				op: BinaryOp::LogicalAnd,
				right
			} => {
				match (&left.val, &right.val) {
					(RawASTNode::Boolean(false), _) => {
						ast.val = RawASTNode::Boolean(false);
					},
					(_, RawASTNode::Boolean(false)) if Self::have_no_effects(left) => {
						ast.val = RawASTNode::Boolean(false);
					},
					(_, RawASTNode::Boolean(true)) => {
						*ast = mem::take(left);
					},
					(RawASTNode::Boolean(true), _) => {
						*ast = mem::take(right);
					},
					_ => {},
				}
			},
			RawASTNode::Binary {
				left,
				op: BinaryOp::LogicalOr,
				right
			} => {
				match (&left.val, &right.val) {
					(RawASTNode::Boolean(true), _) |
					(_, RawASTNode::Boolean(true)) if Self::have_no_effects(left) && Self::have_no_effects(right) => {
						ast.val = RawASTNode::Boolean(true);
					},
					(_, RawASTNode::Boolean(false)) => {
						*ast = mem::take(left);
					},
					(RawASTNode::Boolean(false), _) => {
						*ast = mem::take(right);
					},
					_ => {},
				}
			},
			_ => {},
		}
	}

	pub fn fold(&mut self, ast: &mut ASTNode) -> Result<(), OptimizerError> {
		let span = ast.span;

		Self::simplify(ast);

		match &mut ast.val {
			RawASTNode::None | RawASTNode::Boolean(..) |
			RawASTNode::Float(..) | RawASTNode::String(..) => {},
			RawASTNode::Variable(name) => {
				if let Some(index) = self.unused_vars.iter().position(|v| v == name) {
					self.unused_vars.remove(index);
				}

				// let constant_value = self.constants.get(name).ok_or(OptimizerError::NotKnownAtThisScope(name.to_owned()))?;
				// ast.value = constant_value.clone().try_into()?;
			},
			RawASTNode::Binary { left, op:BinaryOp::Assignment, right } if let RawASTNode::Variable(name) = &left.val => {
				self.fold(right)?;

				self.unused_vars.push(name.to_owned());

				// let Ok(constant_value): Result<UniResult, _> = (**right).clone().try_into() else {
				// 	return Ok(())
				// };

				// self.constants.insert(name.to_owned(), constant_value);

				// ast.value = ASTNodeEnum::None;
			},
			RawASTNode::Binary {
				left: _, op:BinaryOp::PlusAssignment |
							BinaryOp::MinusAssignment |
							BinaryOp::MultiplyAssignment |
							BinaryOp::PowAssignment |
							BinaryOp::DivideAssignment,
				right } => {
				self.fold(right)?;
			},
			RawASTNode::Unary { op: UnaryOp::Minus, value } => {
				self.fold(value)?;

				let Ok(value): Result<UniResult, _> = (**value).clone().try_into() else {
					return Ok(())
				};

				*ast = RawUniResult::calc_unary_by_op(&value, &UnaryOp::Minus).unwrap().try_into().unwrap();
			},
			RawASTNode::Unary { op: UnaryOp::Not, value } => {
				self.fold(value)?;

				let Ok(value): Result<UniResult, _> = (**value).clone().try_into() else {
					return Ok(())
				};

				*ast = RawUniResult::calc_unary_by_op(&value, &UnaryOp::Not).unwrap().try_into().unwrap();
			},
			RawASTNode::Binary { left, op, right } => {
				let left_is_err = self.fold(left).is_err();
				let right_is_err = self.fold(right).is_err();

				if left_is_err || right_is_err {
					return Ok(())
				}

				let Ok(left) = UniResult::try_from(*left.clone()) else {
					return Ok(())
				};

				let Ok(right) = UniResult::try_from(*right.clone()) else {
					return Ok(())
				};

				*ast = match RawUniResult::calc_binary_by_op(&left, &right, op).map(ASTNode::try_from) {
					Ok(Ok(x)) => x,
					Ok(Err(
						UniResultError { val: RawUniResultError::UnsupportedOperation(..), .. }
					)) |
					Err(
						UniResultError { val: RawUniResultError::UnsupportedOperation(..), .. }
					) => return Ok(()),
					Ok(Err(e)) |
					Err(e) => return Err(RawOptimizerError::from(e).into()),
				};
			},
			RawASTNode::Tuple(values) => {
				for value in values {
					self.fold(value)?;
				}
			},
			RawASTNode::Function { name: _, arg } => {
				self.fold(arg)?;
			},
			RawASTNode::FunctionDefinition { name: _, args: _, body } => {
				self.fold(body)?;
			},
			RawASTNode::If { condition, body, else_body: None } => {
				self.fold(body)?;

				self.fold(condition)?;

				let condition_folded: Result<UniResult, _> = (**condition).clone().try_into();

				if let Ok(condition) = condition_folded {
					if bool::from(condition.val) {
						*ast = *body.clone();
					} else {
						ast.val = RawASTNode::None;
					}
				}
			},
			RawASTNode::If { condition, body, else_body: Some(else_body) } => {
				self.fold(condition)?;

				let condition_folded: Result<UniResult, UniResultError> = (**condition).clone().try_into();
				self.fold(body)?;
				self.fold(else_body)?;

				if let Ok(condition) = condition_folded {
					if bool::from(condition.val) {
						*ast = *body.clone();
					} else {
						*ast = *else_body.clone();
					}
				}
			},
			RawASTNode::While { condition, body, else_body: None } => {
				self.fold(body)?;

				self.fold(condition)?;

				let Ok(condition): Result<UniResult, _> = (**condition).clone().try_into() else {
					return Ok(())
				};

				if !bool::from(condition.val) {
					ast.val = RawASTNode::None;
				}
			},
			RawASTNode::While { condition, body, else_body: Some(else_body) } => {
				self.fold(condition)?;
				self.fold(body)?;
				self.fold(else_body)?;

				let Ok(condition): Result<UniResult, _> = (**condition).clone().try_into() else {
					return Ok(())
				};

				if !bool::from(condition.val) {
					ast.val = RawASTNode::None;
				}
			},
			RawASTNode::Break(value) |
			RawASTNode::Return(value) => {
				self.fold(value)?;
			},
			RawASTNode::Block(exprs) => {
				for expr in exprs {
					self.fold(expr)?;
				}
			},
		}

		Ok(())
	}
}
