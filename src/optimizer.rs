use std::collections::VecDeque;

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

	pub fn fold(ast: ASTNodeEnum) -> Result<ASTNodeEnum, OptimizerError> {
		let result = match ast {
			ASTNodeEnum::Binary { left, op: TokenType::TokenAssignment, right } => {
				let right = Box::new(Self::fold(right.value)?.into());

				ASTNodeEnum::Binary { left, op: TokenType::TokenAssignment, right }
			},
			ASTNodeEnum::Binary { left, op: TokenType::TokenPlusAssignment, right } => {
				let right = Box::new(Self::fold(right.value)?.into());

				ASTNodeEnum::Binary { left, op: TokenType::TokenPlusAssignment, right }
			},
			ASTNodeEnum::Binary { left, op: TokenType::TokenMinusAssignment, right } => {
				let right = Box::new(Self::fold(right.value)?.into());

				ASTNodeEnum::Binary { left, op: TokenType::TokenMinusAssignment, right }
			},
			ASTNodeEnum::Binary { left, op: TokenType::TokenMultiplyAssignment, right } => {
				let right = Box::new(Self::fold(right.value)?.into());

				ASTNodeEnum::Binary { left, op: TokenType::TokenMultiplyAssignment, right }
			},
			ASTNodeEnum::Binary { left, op: TokenType::TokenDivideAssignment, right } => {
				let right = Box::new(Self::fold(right.value)?.into());

				ASTNodeEnum::Binary { left, op: TokenType::TokenDivideAssignment, right }
			},
			ASTNodeEnum::Binary { left, op, right } => {
				let left_folded = Self::fold(left.value.clone());
				let right_folded = Self::fold(right.value.clone());

				match (left_folded, right_folded) {
					(Ok(left), Ok(right)) => {
						calc_binary_by_op(&left.try_into()?, &right.try_into()?, op)?.try_into()?
					},
					(Err(_), Ok(right)) => {
						right
					},
					(Ok(left), Err(_)) => {
						left
					},
					(Err(_), Err(_)) => {
						ASTNodeEnum::Binary { left, op, right }
					},
				}
			},
			ASTNodeEnum::Tuple(values) => {
				let mut result = Vec::new();

				for value in values {
					result.push(Self::fold(value.value)?.into());
				}

				ASTNodeEnum::Tuple(result)
			},
			ASTNodeEnum::Function { name, arg } => {
				let arg = Box::new(Self::fold(arg.value)?.into());

				ASTNodeEnum::Function { name, arg }
			},
			ASTNodeEnum::FunctionDefinition { name, args, block } => {
				let block = Box::new(Self::fold(block.value)?.into());

				ASTNodeEnum::FunctionDefinition { name, args, block }
			},
			ASTNodeEnum::If { condition, block, block_else: None } => {
				let condition = Box::new(Self::fold(condition.value)?.into());
				let block = Box::new(Self::fold(block.value)?.into());

				ASTNodeEnum::If { condition, block, block_else: None }
			},
			ASTNodeEnum::If { condition, block, block_else: Some(block_else) } => {
				let condition = Box::new(Self::fold(condition.value)?.into());
				let block = Box::new(Self::fold(block.value)?.into());
				let block_else = Box::new(Self::fold(block_else.value)?.into());

				ASTNodeEnum::If { condition, block, block_else: Some(block_else) }
			},
			ASTNodeEnum::While { condition, block, block_else: None } => {
				let condition = Box::new(Self::fold(condition.value)?.into());
				let block = Box::new(Self::fold(block.value)?.into());

				ASTNodeEnum::While { condition, block, block_else: None }
			},
			ASTNodeEnum::While { condition, block, block_else: Some(block_else) } => {
				let condition = Box::new(Self::fold(condition.value)?.into());
				let block = Box::new(Self::fold(block.value)?.into());
				let block_else = Box::new(Self::fold(block_else.value)?.into());

				ASTNodeEnum::While { condition, block, block_else: Some(block_else) }
			},
			ASTNodeEnum::Block(exprs) => {
				let mut result = VecDeque::new();

				for expr in exprs {
					result.push_back(Self::fold(expr.value)?.into());
				}

				ASTNodeEnum::Block(result)
			},
			_ => ast,
		};

		Ok(result)
	}
}
