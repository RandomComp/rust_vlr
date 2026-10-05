use std::mem;

use crate::{parser::{UnaryOp, BinaryOp, ASTNode, ASTNodeEnum}, uni_result::{UniResult, UniResultError, calc_binary_by_op}};

#[derive(thiserror::Error, Debug)]
pub enum OptimizerError {
	#[error("{0}")]
	UniResultError(#[from] UniResultError),
}

pub struct Optimizer {
}

impl Optimizer {
	// pub fn new() -> Self {
	// 	Self {}
	// }

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
				op: BinaryOp::Plus | BinaryOp::Minus,
				right
			} if let ASTNodeEnum::Number(0.0) = right.value => {
				 *ast = mem::take(left);
			},
			// ASTNodeEnum::Binary {
			// 	left,
			// 	op: TokenType::TokenMinus,
			// 	right
			// } if left => {
			// 	*ast = ASTNode { value: ASTNodeEnum::Number(0.0), row, column };
			// },
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

	pub fn fold(ast: &mut ASTNode) -> Result<(), OptimizerError> {
		let (row, column) = (ast.row, ast.column);

		match &mut ast.value {
			ASTNodeEnum::Binary {
				left: _, op:BinaryOp::Assignment |
							BinaryOp::PlusAssignment |
							BinaryOp::MinusAssignment |
							BinaryOp::MultiplyAssignment |
							BinaryOp::PowAssignment |
							BinaryOp::DivideAssignment,
				right } => {
				Self::fold(right)?;
			},
			ASTNodeEnum::Unary { op: UnaryOp::Minus, value } => {
				Self::fold(value)?;

				let Ok(value): Result<UniResult, _> = (**value).clone().try_into() else {
					Self::simplify(ast);

					return Ok(())
				};

				ast.value = value.neg()?.try_into()?;
			},
			ASTNodeEnum::Unary { op: UnaryOp::Not, value } => {
				Self::fold(value)?;

				let Ok(value): Result<UniResult, _> = (**value).clone().try_into() else {
					Self::simplify(ast);

					return Ok(())
				};

				ast.value = value.not()?.try_into()?;
			},
			ASTNodeEnum::Binary { left, op, right } => {
				let left_is_err = Self::fold(left).is_err();
				let right_is_err = Self::fold(right).is_err();

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
					Self::fold(value)?;
				}
			},
			ASTNodeEnum::Function { name: _, arg } => {
				Self::fold(arg)?;
			},
			ASTNodeEnum::FunctionDefinition { name: _, args: _, block } => {
				Self::fold(block)?;
			},
			ASTNodeEnum::If { condition, block, block_else: None } => {
				Self::fold(block)?;

				Self::fold(condition)?;

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
				Self::fold(condition)?;

				let condition_folded: Result<UniResult, UniResultError> = (**condition).clone().try_into();
				Self::fold(block)?;
				Self::fold(block_else)?;

				if let Ok(condition) = condition_folded {
					if condition.to_bool()? {
						*ast = *block.clone();
					} else {
						*ast = *block_else.clone();
					}
				}
			},
			ASTNodeEnum::While { condition, block, block_else: None } => {
				Self::fold(block)?;

				Self::fold(condition)?;

				let Ok(condition): Result<UniResult, _> = (**condition).clone().try_into() else {
					return Ok(())
				};

				if !condition.to_bool()? {
					*ast = ASTNode { value: ASTNodeEnum::None, row, column };
				}
			},
			ASTNodeEnum::While { condition, block, block_else: Some(block_else) } => {
				Self::fold(condition)?;
				Self::fold(block)?;
				Self::fold(block_else)?;

				let Ok(condition): Result<UniResult, _> = (**condition).clone().try_into() else {
					return Ok(())
				};

				if !condition.to_bool()? {
					*ast = ASTNode { value: ASTNodeEnum::None, row, column };
				}
			},
			ASTNodeEnum::Block(exprs) => {
				for expr in exprs {
					Self::fold(expr)?;
				}
			},
			_ => {},
		}

		Ok(())
	}
}
