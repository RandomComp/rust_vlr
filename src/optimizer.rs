use std::{collections::HashMap, mem};

use crate::{parser::{UnaryOp, BinaryOp, ASTNode, ASTNode}, uni_type::{UniResult, UniResultError}};

#[derive(thiserror::Error, Debug)]
pub enum OptimizerError {
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
		match &ast.value {
			ASTNode::Function { .. } |
			ASTNode::FunctionDefinition { name: Some(_), .. } |
			ASTNode::Binary {
				left: _,
				op: BinaryOp::Assignment |
					BinaryOp::PlusAssignment |
					BinaryOp::MinusAssignment |
					BinaryOp::MultiplyAssignment |
					BinaryOp::PowAssignment |
					BinaryOp::DivideAssignment,
				right: _
			} => false,
			ASTNode::Binary { left, op: BinaryOp::LogicalAnd | BinaryOp::LogicalOr, right } => {
				Self::have_no_effects(left) && Self::have_no_effects(right)
			},
			_ => true,
		}
	}

	pub fn simplify(ast: &mut ASTNode) {
		let (row, column) = (ast.row, ast.column);

		match &mut ast.value {
			ASTNode::Binary {
				left,
				op: BinaryOp::Multiply,
				right
			} if left.value == ASTNode::Float(0.0) || right.value == ASTNode::Float(0.0) => {
				ast.value = ASTNode::Float(0.0);
			},
			ASTNode::Binary {
				left,
				op: BinaryOp::Multiply,
				right
			} if right.value == ASTNode::Float(1.0) => {
				 *ast = mem::take(left);
			},
			ASTNode::Binary {
				left,
				op: BinaryOp::Multiply,
				right
			} if left.value == ASTNode::Float(1.0) => {
				 *ast = mem::take(right);
			},
			ASTNode::Binary {
				left,
				op: BinaryOp::Plus,
				right
			} if let ASTNode::Float(0.0) = left.value => {
				 *ast = mem::take(right);
			},
			ASTNode::Binary {
				left,
				op: BinaryOp::Minus,
				right
			} if let ASTNode::Float(0.0) = left.value => {
				*ast = ASTNode {
					value: ASTNode::Unary {
						op: UnaryOp::Minus, value: mem::take(right)
					},
					row, column
				};
			},
			ASTNode::Binary {
				left,
				op: BinaryOp::Plus | BinaryOp::Minus,
				right
			} if let ASTNode::Float(0.0) = right.value => {
				 *ast = mem::take(left);
			},
			ASTNode::Binary {
				left,
				op: BinaryOp::Plus,
				right
			} if left.value == right.value => {
				*ast = ASTNode {
					value: ASTNode::Binary {
						left: mem::take(left),
						op: BinaryOp::Multiply,
						right: Box::new(ASTNode { value: ASTNode::Float(2.0), row, column }),
					},
					row, column
				};
			},
			ASTNode::Binary {
				left,
				op: BinaryOp::Minus,
				right
			} if left.value == right.value => {
				ast.value = ASTNode::Float(0.0);
			},
			ASTNode::Binary {
				left,
				op: BinaryOp::Equals | BinaryOp::LessOrEquals | BinaryOp::GreatOrEquals,
				right
			} if left.value == right.value => {
				ast.value = ASTNode::Boolean(true);
			},
			ASTNode::Binary {
				left,
				op: BinaryOp::Neq | BinaryOp::Less | BinaryOp::Great,
				right
			} if left.value == right.value => {
				ast.value = ASTNode::Boolean(false);
			},
			ASTNode::Binary {
				left,
				op: BinaryOp::LogicalAnd,
				right
			} => {
				match (&left.value, &right.value) {
					(ASTNode::Boolean(false), _) => {
						ast.value = ASTNode::Boolean(false);
					},
					(_, ASTNode::Boolean(false)) if Self::have_no_effects(left) => {
						ast.value = ASTNode::Boolean(false);
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
				op: BinaryOp::LogicalOr,
				right
			} => {
				match (&left.value, &right.value) {
					(ASTNode::Boolean(true), _) |
					(_, ASTNode::Boolean(true)) if Self::have_no_effects(left) && Self::have_no_effects(right) => {
						ast.value = ASTNode::Boolean(true);
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

	pub fn fold(&mut self, ast: &mut ASTNode) -> Result<(), OptimizerError> {
		let (row, column) = (ast.row, ast.column);

		Self::simplify(ast);

		match &mut ast.value {
			ASTNode::None | ASTNode::Boolean(..) |
			ASTNode::Float(..) | ASTNode::String(..) => {},
			ASTNode::Variable(name) => {
				if let Some(index) = self.unused_vars.iter().position(|v| v == name) {
					self.unused_vars.remove(index);
				}

				// let constant_value = self.constants.get(name).ok_or(OptimizerError::NotKnownAtThisScope(name.to_owned()))?;
				// ast.value = constant_value.clone().try_into()?;
			},
			ASTNode::Binary { left, op:BinaryOp::Assignment, right } if let ASTNode::Variable(name) = &left.value => {
				self.fold(right)?;

				self.unused_vars.push(name.to_owned());

				// let Ok(constant_value): Result<UniResult, _> = (**right).clone().try_into() else {
				// 	return Ok(())
				// };

				// self.constants.insert(name.to_owned(), constant_value);

				// ast.value = ASTNodeEnum::None;
			},
			ASTNode::Binary {
				left: _, op:BinaryOp::PlusAssignment |
							BinaryOp::MinusAssignment |
							BinaryOp::MultiplyAssignment |
							BinaryOp::PowAssignment |
							BinaryOp::DivideAssignment,
				right } => {
				self.fold(right)?;
			},
			ASTNode::Unary { op: UnaryOp::Minus, value } => {
				self.fold(value)?;

				let Ok(value): Result<UniResult, _> = (**value).clone().try_into() else {
					return Ok(())
				};

				ast.value = value.neg()?.try_into()?;
			},
			ASTNode::Unary { op: UnaryOp::Not, value } => {
				self.fold(value)?;

				let Ok(value): Result<UniResult, _> = (**value).clone().try_into() else {
					return Ok(())
				};

				ast.value = value.not()?.try_into()?;
			},
			ASTNode::Binary { left, op, right } => {
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

				*ast = match left.calc_binary_by_op(&right, op).map(UniResult::try_into) {
					Ok(Ok(x)) => x,
					Ok(Err(UniResultError::UnsupportedOperation(..))) |
					Err(UniResultError::UnsupportedOperation(..)) => return Ok(()),
					Ok(Err(e)) |
					Err(e) => return Err(e.into()),
				};
			},
			ASTNode::Tuple(values) => {
				for value in values {
					self.fold(value)?;
				}
			},
			ASTNode::Function { name: _, arg } => {
				self.fold(arg)?;
			},
			ASTNode::FunctionDefinition { name: _, args: _, body } => {
				self.fold(body)?;
			},
			ASTNode::If { condition, body, else_body: None } => {
				self.fold(body)?;

				self.fold(condition)?;

				let condition_folded: Result<UniResult, _> = (**condition).clone().try_into();

				if let Ok(condition) = condition_folded {
					if condition.to_bool()? {
						*ast = *body.clone();
					} else {
						ast.value = ASTNode::None;
					}
				}
			},
			ASTNode::If { condition, body, else_body: Some(else_body) } => {
				self.fold(condition)?;

				let condition_folded: Result<UniResult, UniResultError> = (**condition).clone().try_into();
				self.fold(body)?;
				self.fold(else_body)?;

				if let Ok(condition) = condition_folded {
					if condition.to_bool()? {
						*ast = *body.clone();
					} else {
						*ast = *else_body.clone();
					}
				}
			},
			ASTNode::While { condition, body, else_body: None } => {
				self.fold(body)?;

				self.fold(condition)?;

				let Ok(condition): Result<UniResult, _> = (**condition).clone().try_into() else {
					return Ok(())
				};

				if !condition.to_bool()? {
					*ast = ASTNode { value: ASTNode::None, row, column };
				}
			},
			ASTNode::While { condition, body, else_body: Some(else_body) } => {
				self.fold(condition)?;
				self.fold(body)?;
				self.fold(else_body)?;

				let Ok(condition): Result<UniResult, _> = (**condition).clone().try_into() else {
					return Ok(())
				};

				if !condition.to_bool()? {
					*ast = ASTNode { value: ASTNode::None, row, column };
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
