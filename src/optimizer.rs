use std::{collections::HashMap, mem};

use crate::{parser::{UnaryOp, BinaryOp, ASTNode, ASTNodeEnum}, uni_type::{UniResult, UniResultError, calc_binary_by_op}};

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
			ASTNodeEnum::Function { .. } |
			ASTNodeEnum::FunctionDefinition { name: Some(_), .. } |
			ASTNodeEnum::Binary {
				left: _,
				op: BinaryOp::Assignment |
					BinaryOp::PlusAssignment |
					BinaryOp::MinusAssignment |
					BinaryOp::MultiplyAssignment |
					BinaryOp::PowAssignment |
					BinaryOp::DivideAssignment,
				right: _
			} => false,
			ASTNodeEnum::Binary { left, op: BinaryOp::LogicalAnd | BinaryOp::LogicalOr, right } => {
				Self::have_no_effects(left) && Self::have_no_effects(right)
			},
			_ => true,
		}
	}

	pub fn simplify(ast: &mut ASTNode) {
		let (row, column) = (ast.row, ast.column);

		match &mut ast.value {
			ASTNodeEnum::Binary {
				left,
				op: BinaryOp::Multiply,
				right
			} if left.value == ASTNodeEnum::Float(0.0) || right.value == ASTNodeEnum::Float(0.0) => {
				ast.value = ASTNodeEnum::Float(0.0);
			},
			ASTNodeEnum::Binary {
				left,
				op: BinaryOp::Multiply,
				right
			} if right.value == ASTNodeEnum::Float(1.0) => {
				 *ast = mem::take(left);
			},
			ASTNodeEnum::Binary {
				left,
				op: BinaryOp::Multiply,
				right
			} if left.value == ASTNodeEnum::Float(1.0) => {
				 *ast = mem::take(right);
			},
			ASTNodeEnum::Binary {
				left,
				op: BinaryOp::Plus,
				right
			} if let ASTNodeEnum::Float(0.0) = left.value => {
				 *ast = mem::take(right);
			},
			ASTNodeEnum::Binary {
				left,
				op: BinaryOp::Minus,
				right
			} if let ASTNodeEnum::Float(0.0) = left.value => {
				*ast = ASTNode {
					value: ASTNodeEnum::Unary {
						op: UnaryOp::Minus, value: mem::take(right)
					},
					row, column
				};
			},
			ASTNodeEnum::Binary {
				left,
				op: BinaryOp::Plus | BinaryOp::Minus,
				right
			} if let ASTNodeEnum::Float(0.0) = right.value => {
				 *ast = mem::take(left);
			},
			ASTNodeEnum::Binary {
				left,
				op: BinaryOp::Plus,
				right
			} if left.value == right.value => {
				*ast = ASTNode {
					value: ASTNodeEnum::Binary {
						left: mem::take(left),
						op: BinaryOp::Multiply,
						right: Box::new(ASTNode { value: ASTNodeEnum::Float(2.0), row, column }),
					},
					row, column
				};
			},
			ASTNodeEnum::Binary {
				left,
				op: BinaryOp::Minus,
				right
			} if left.value == right.value => {
				ast.value = ASTNodeEnum::Float(0.0);
			},
			ASTNodeEnum::Binary {
				left,
				op: BinaryOp::Equals | BinaryOp::LessOrEquals | BinaryOp::GreatOrEquals,
				right
			} if left.value == right.value => {
				ast.value = ASTNodeEnum::Boolean(true);
			},
			ASTNodeEnum::Binary {
				left,
				op: BinaryOp::NotEquals | BinaryOp::Less | BinaryOp::Great,
				right
			} if left.value == right.value => {
				ast.value = ASTNodeEnum::Boolean(false);
			},
			ASTNodeEnum::Binary {
				left,
				op: BinaryOp::LogicalAnd,
				right
			} => {
				match (&left.value, &right.value) {
					(ASTNodeEnum::Boolean(false), _) => {
						ast.value = ASTNodeEnum::Boolean(false);
					},
					(_, ASTNodeEnum::Boolean(false)) if Self::have_no_effects(left) => {
						ast.value = ASTNodeEnum::Boolean(false);
					},
					(_, ASTNodeEnum::Boolean(true)) => {
						*ast = mem::take(left);
					},
					(ASTNodeEnum::Boolean(true), _) => {
						*ast = mem::take(right);
					},
					_ => {},
				}
			},
			ASTNodeEnum::Binary {
				left,
				op: BinaryOp::LogicalOr,
				right
			} => {
				match (&left.value, &right.value) {
					(ASTNodeEnum::Boolean(true), _) |
					(_, ASTNodeEnum::Boolean(true)) if Self::have_no_effects(left) && Self::have_no_effects(right) => {
						ast.value = ASTNodeEnum::Boolean(true);
					},
					(_, ASTNodeEnum::Boolean(false)) => {
						*ast = mem::take(left);
					},
					(ASTNodeEnum::Boolean(false), _) => {
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

		match &mut ast.value {
			ASTNodeEnum::Variable(name) => {
				if let Some(index) = self.unused_vars.iter().position(|v| v == name) {
					self.unused_vars.remove(index);
				}

				// let constant_value = self.constants.get(name).ok_or(OptimizerError::NotKnownAtThisScope(name.to_owned()))?;
				// ast.value = constant_value.clone().try_into()?;
			},
			ASTNodeEnum::Binary { left, op:BinaryOp::Assignment, right } if let ASTNodeEnum::Variable(name) = &left.value => {
				self.fold(right)?;

				self.unused_vars.push(name.to_owned());

				// let Ok(constant_value): Result<UniResult, _> = (**right).clone().try_into() else {
				// 	return Ok(())
				// };

				// self.constants.insert(name.to_owned(), constant_value);

				// ast.value = ASTNodeEnum::None;
			},
			ASTNodeEnum::Binary {
				left: _, op:BinaryOp::PlusAssignment |
							BinaryOp::MinusAssignment |
							BinaryOp::MultiplyAssignment |
							BinaryOp::PowAssignment |
							BinaryOp::DivideAssignment,
				right } => {
				self.fold(right)?;
			},
			ASTNodeEnum::Unary { op: UnaryOp::Minus, value } => {
				self.fold(value)?;

				let Ok(value): Result<UniResult, _> = (**value).clone().try_into() else {
					Self::simplify(ast);

					return Ok(())
				};

				ast.value = value.neg()?.try_into()?;
			},
			ASTNodeEnum::Unary { op: UnaryOp::Not, value } => {
				self.fold(value)?;

				let Ok(value): Result<UniResult, _> = (**value).clone().try_into() else {
					Self::simplify(ast);

					return Ok(())
				};

				ast.value = value.not()?.try_into()?;
			},
			ASTNodeEnum::Binary { left, op, right } => {
				let left_is_err = self.fold(left).is_err();
				let right_is_err = self.fold(right).is_err();

				if left_is_err || right_is_err {
					Self::simplify(ast);

					return Ok(())
				}

				let Ok(left) = &(**left).clone().try_into() else {
					Self::simplify(ast);

					return Ok(())
				};

				let Ok(right) = &(**right).clone().try_into() else {
					Self::simplify(ast);

					return Ok(())
				};

				*ast = calc_binary_by_op(left, right, op)?.try_into()?;
			},
			ASTNodeEnum::Tuple(values) => {
				for value in values {
					self.fold(value)?;
				}
			},
			ASTNodeEnum::Function { name: _, arg } => {
				self.fold(arg)?;
			},
			ASTNodeEnum::FunctionDefinition { name: _, args: _, block } => {
				self.fold(block)?;
			},
			ASTNodeEnum::If { condition, block, block_else: None } => {
				self.fold(block)?;

				self.fold(condition)?;

				let condition_folded: Result<UniResult, _> = (**condition).clone().try_into();

				if let Ok(condition) = condition_folded {
					if condition.to_bool()? {
						*ast = *block.clone();
					} else {
						ast.value = ASTNodeEnum::None;
					}
				}
			},
			ASTNodeEnum::If { condition, block, block_else: Some(block_else) } => {
				self.fold(condition)?;

				let condition_folded: Result<UniResult, UniResultError> = (**condition).clone().try_into();
				self.fold(block)?;
				self.fold(block_else)?;

				if let Ok(condition) = condition_folded {
					if condition.to_bool()? {
						*ast = *block.clone();
					} else {
						*ast = *block_else.clone();
					}
				}
			},
			ASTNodeEnum::While { condition, block, block_else: None } => {
				self.fold(block)?;

				self.fold(condition)?;

				let Ok(condition): Result<UniResult, _> = (**condition).clone().try_into() else {
					return Ok(())
				};

				if !condition.to_bool()? {
					*ast = ASTNode { value: ASTNodeEnum::None, row, column };
				}
			},
			ASTNodeEnum::While { condition, block, block_else: Some(block_else) } => {
				self.fold(condition)?;
				self.fold(block)?;
				self.fold(block_else)?;

				let Ok(condition): Result<UniResult, _> = (**condition).clone().try_into() else {
					return Ok(())
				};

				if !condition.to_bool()? {
					*ast = ASTNode { value: ASTNodeEnum::None, row, column };
				}
			},
			ASTNodeEnum::Block(exprs) => {
				for expr in exprs {
					self.fold(expr)?;
				}
			},
			_ => {},
		}

		Ok(())
	}
}
