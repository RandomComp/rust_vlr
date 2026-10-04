use std::collections::{HashMap, VecDeque};

use crate::{lexer::TokenType, parser::{ASTNode, ASTNodeEnum}, uni_result::{UniResult, UniResultError, calc_binary_by_op}};

#[derive(thiserror::Error, Debug)]
pub enum OptimizerError {
	#[error("UniResult error: {0}")]
	UniResultError(#[from] UniResultError),
}

pub struct Optimizer {
}

impl Optimizer {
	// pub fn new() -> Self {
	// 	Self {}
	// }

	pub fn fold(ast: &ASTNode) -> Result<ASTNode, OptimizerError> {
		let (row, column) = (ast.row, ast.column);

		let result = match &ast.value {
			ASTNodeEnum::Binary { left, op: TokenType::TokenAssignment, right } => {
				let right = Box::new(Self::fold(&right)?);

				ASTNode { value: ASTNodeEnum::Binary { left: left.to_owned(), op: TokenType::TokenAssignment, right: right.to_owned() }, row, column }
			},
			ASTNodeEnum::Binary { left, op: TokenType::TokenPlusAssignment, right } => {
				let right = Box::new(Self::fold(&right)?);

				ASTNode { value: ASTNodeEnum::Binary { left: left.to_owned(), op: TokenType::TokenPlusAssignment, right: right.to_owned() }, row, column }
			},
			ASTNodeEnum::Binary { left, op: TokenType::TokenMinusAssignment, right } => {
				let right = Box::new(Self::fold(&right)?);

				ASTNode { value: ASTNodeEnum::Binary { left: left.to_owned(), op: TokenType::TokenMinusAssignment, right: right.to_owned() }, row, column }
			},
			ASTNodeEnum::Binary { left, op: TokenType::TokenMultiplyAssignment, right } => {
				let right = Box::new(Self::fold(&right)?);

				ASTNode { value: ASTNodeEnum::Binary { left: left.to_owned(), op: TokenType::TokenMultiplyAssignment, right: right.to_owned() }, row, column }
			},
			ASTNodeEnum::Binary { left, op: TokenType::TokenDivideAssignment, right } => {
				let right = Box::new(Self::fold(&right)?);

				ASTNode { value: ASTNodeEnum::Binary { left: left.to_owned(), op: TokenType::TokenDivideAssignment, right: right.to_owned() }, row, column }
			},
			ASTNodeEnum::Binary { left, op, right } => {
				let left_folded = Self::fold(left);
				let right_folded = Self::fold(right);

				let Ok(left) = left_folded else {
					return Ok(ASTNode { value: ASTNodeEnum::Binary { left: left.to_owned(), op: op.to_owned(), right: right.to_owned() }, row, column })
				};

				let Ok(right) = right_folded else {
					return Ok(ASTNode { value: ASTNodeEnum::Binary { left: Box::new(left), op: op.to_owned(), right: right.to_owned() }, row, column })
				};

				let result = match (left.clone().try_into() as Result<UniResult, _>, right.clone().try_into() as Result<UniResult, _>) {
					(Ok(left), Ok(right)) =>
						calc_binary_by_op(&left, &right, op)?.try_into()?,
					(Ok(left), Err(_)) =>
						ASTNodeEnum::Binary {
							left: Box::new(ASTNode { value: left.try_into()?, row, column }),
							op: op.to_owned(),
							right: Box::new(right)
						},
					(Err(_), Ok(right)) =>
						ASTNodeEnum::Binary {
							left: Box::new(left),
							op: op.to_owned(),
							right: Box::new(ASTNode { value: right.try_into()?, row, column })
						},
					(Err(_), Err(_)) =>
						ASTNodeEnum::Binary {
							left: Box::new(ASTNode { value: left.value, row, column }),
							op: op.to_owned(),
							right: Box::new(ASTNode { value: right.value, row, column })
						},
				};

				ASTNode { value: result, row, column }
			},
			ASTNodeEnum::Tuple(values) => {
				let mut result = Vec::new();

				for value in values {
					result.push(Self::fold(&value)?);
				}

				ASTNode { value: ASTNodeEnum::Tuple(result), row, column }
			},
			ASTNodeEnum::Function { name, arg } => {
				let arg = Box::new(Self::fold(&arg)?);

				ASTNode { value: ASTNodeEnum::Function { name: name.to_owned(), arg }, row, column }
			},
			ASTNodeEnum::FunctionDefinition { name, args, block } => {
				let block = Box::new(Self::fold(block)?);

				ASTNode { value: ASTNodeEnum::FunctionDefinition { name: name.to_owned(), args: args.to_owned(), block }, row, column }
			},
			ASTNodeEnum::If { condition, block, block_else: None } => {
				let block = Self::fold(&*block)?;

				let condition_folded: Result<UniResult, UniResultError> = Self::fold(&*condition)?.try_into();

				if let Ok(condition) = condition_folded {
					if condition.to_bool()? {
						block
					} else {
						ASTNode { value: ASTNodeEnum::None, row, column }
					}
				} else {
					ASTNode { value: ASTNodeEnum::If { condition: condition.to_owned(), block: Box::new(block), block_else: None }, row, column }
				}
			},
			ASTNodeEnum::If { condition, block, block_else: Some(block_else) } => {
				let condition_folded: Result<UniResult, UniResultError> = Self::fold(condition)?.try_into();
				let block = Self::fold(&*block)?;
				let block_else = Self::fold(&*block_else)?;

				if let Ok(condition) = condition_folded {
					if condition.to_bool()? {
						block
					} else {
						block_else
					}
				} else {
					ASTNode { value: ASTNodeEnum::If { condition: condition.to_owned(), block: Box::new(block), block_else: Some(Box::new(block_else)) }, row, column }
				}
			},
			ASTNodeEnum::While { condition, block, block_else: None } => {
				let block = Self::fold(&*block)?;

				let Ok(condition): Result<UniResult, _> = Self::fold(&*condition)?.try_into() else {
					return Ok(ASTNode { value: ASTNodeEnum::While { condition: condition.to_owned(), block: Box::new(block), block_else: None }, row, column })
				};

				if condition.to_bool()? {
					ASTNode {
						value: ASTNodeEnum::While {
							condition: Box::new(ASTNodeEnum::Boolean(true).into()),
							block: Box::new(block),
							block_else: None,
						},
						row, column
					}
				} else {
					ASTNode { value: ASTNodeEnum::None, row, column }
				}

				// let condition = Box::new(Self::fold(*condition)?);
				// let block = Box::new(Self::fold(*block)?);

				// ASTNode { value: ASTNodeEnum::While { condition, block, block_else: None }, row, column }
			},
			ASTNodeEnum::While { condition, block, block_else: Some(block_else) } => {
				let condition = Box::new(Self::fold(&*condition)?);
				let block = Box::new(Self::fold(&*block)?);
				let block_else = Box::new(Self::fold(&*block_else)?);

				ASTNode { value: ASTNodeEnum::While { condition, block, block_else: Some(block_else) }, row, column }
			},
			ASTNodeEnum::Block(exprs) => {
				let mut result = VecDeque::new();

				for expr in exprs {
					let expr_folded: ASTNode = Self::fold(&expr)?;

					if let ASTNodeEnum::None = expr_folded.value {
						continue;
					}

					result.push_back(expr_folded);
				}

				ASTNode { value: ASTNodeEnum::Block(result), row, column }
			},
			_ => ast.to_owned(),
		};

		Ok(result)
	}
}
