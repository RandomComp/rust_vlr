use std::fmt::Write;

use crate::compiler::parser::ASTNode;

impl std::fmt::Display for ASTNode {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			ASTNode::None => write!(f, "()"),
			ASTNode::Boolean(value) => write!(f, "{value}"),
			ASTNode::Number(value) => write!(f, "{value}"),
			ASTNode::Float(value) => write!(f, "{value}"),
			ASTNode::String(text) => write!(f, "\"{text}\""),
			ASTNode::Variable(name) => write!(f, "{name}"),
			ASTNode::List(values) => {
				write!(f, "[")?;

				for (i, value) in values.iter().enumerate() {
					if i > 0 {
						write!(f, ", ")?;
					}

					write!(f, "{value}")?;
				}

				write!(f, "]")
			},
			ASTNode::Block(values) => {
				write!(f, "{{ ")?;

				for (i, value) in values.iter().enumerate() {
					if i > 0 {
						write!(f, "; ")?;
					}

					write!(f, "{value}")?;
				}

				write!(f, "}}")?;

				Ok(())
			},
			ASTNode::Function {name, args} => {
				write!(f, "{name}(")?;

				for (i, arg) in args.iter().enumerate() {
					if i > 0 {
						write!(f, ", ")?;
					}

					write!(f, "{arg}")?;
				}

				write!(f, ")")
			},
			ASTNode::If { condition, body, else_body: Some(else_body) } => {
				write!(f, "if {condition} {body} else {else_body}")
			},
			ASTNode::If { condition, body, else_body: None } => {
				write!(f, "if {condition} {body}")
			},
			ASTNode::Break(value) => {
				write!(f, "break {value}")
			},
			ASTNode::Return(value) => {
				write!(f, "return {value}")
			},
			ASTNode::While { condition, body, else_body: Some(else_body) } => {
				write!(f, "while {condition} {body} else {else_body}")
			},
			ASTNode::While { condition, body, else_body: None } => {
				write!(f, "while {condition} {body}")
			},
			// ASTNodeEnum::Ternary { left, op, center, right } =>
			// 	write!(f, "({} {} {} {} {})", left, op, center, op, right),
			ASTNode::Binary { left, op, right } =>
				write!(f, "({left} {op} {right})"),
			ASTNode::Unary {op, value} =>
				write!(f, "{op}({value})"),
			ASTNode::FunctionDefinition { name: Some(name), args, body, is_global } => {
				if *is_global {
					write!(f, "global def {name}(")?;
				} else {
					write!(f, "def {name}(")?;
				}

				for (i, arg) in args.iter().enumerate() {
					if i > 0 {
						write!(f, ", ")?;
					}

					write!(f, "{arg}")?;
				}

				write!(f, ") {body}")?;

				Ok(())
			},
			ASTNode::FunctionDefinition { name: None, args, body: _, is_global: false } => {
				write!(f, "anonymous def(")?;

				for (i, arg) in args.iter().enumerate() {
					if i > 0 {
						write!(f, ", ")?;
					}

					write!(f, "{arg}")?;
				}

				Ok(())
			},
			ASTNode::FunctionDefinition { name: None, args: _, body: _, is_global: true } => {
				unreachable!("invalid combination of is_global for anonymous function")
			},
		}
	}
}

impl ASTNode {
	pub fn format_human_readable(&self, f: &mut String, for_statement: bool, tab_level: usize) -> std::fmt::Result {
		let tab_level_syms: String = std::iter::repeat_n(' ', 4 * tab_level).collect();

		if !for_statement {
			write!(f, "{tab_level_syms}")?;
		}

		match self {
			ASTNode::None => write!(f, "()"),
			ASTNode::Boolean(value) => write!(f, "{value}"),
			ASTNode::Number(value) => write!(f, "{value}"),
			ASTNode::Float(value) => write!(f, "{value}"),
			ASTNode::String(text) => write!(f, "\"{text}\""),
			ASTNode::Variable(name) => write!(f, "{name}"),
			ASTNode::List(values) => {
				write!(f, "[")?;

				for (i, value) in values.iter().enumerate() {
					if i > 0 {
						write!(f, ", ")?;
					}

					value.format_human_readable(f, false, 0)?;
				}

				write!(f, "]")
			},
			ASTNode::Block(values) => {
				writeln!(f, "{{")?;

				for (i, value) in values.iter().enumerate() {
					if i > 0 {
						writeln!(f, ";")?;
					}

					value.format_human_readable(f, false, tab_level + 1)?;
				}

				write!(f, "\n{tab_level_syms}}}")
			},
			ASTNode::If { condition, body, else_body: Some(else_body) } => {
				write!(f, "if ")?;

				condition.format_human_readable(f, false, 0)?;

				write!(f, " ")?;

				body.format_human_readable(f, true, tab_level + 1)?;

				write!(f, " else ")?;

				write!(f, " ")?;

				else_body.format_human_readable(f, true, tab_level + 1)
			},
			ASTNode::If { condition, body, else_body: None } => {
				write!(f, "if ")?;

				condition.format_human_readable(f, false, 0)?;

				write!(f, " ")?;

				body.format_human_readable(f, true, tab_level)
			},
			ASTNode::Break(value) => {
				write!(f, "break {value}")
			},
			ASTNode::Return(value) => {
				write!(f, "return {value}")
			},
			ASTNode::While { condition, body, else_body: Some(else_body) } => {
				write!(f, "while ")?;

				condition.format_human_readable(f, false, 0)?;

				write!(f, " ")?;

				body.format_human_readable(f, false, tab_level + 1)?;

				write!(f, " else ")?;

				write!(f, " ")?;

				else_body.format_human_readable(f, true, tab_level + 1)
			},
			ASTNode::While { condition, body, else_body: None } => {
				write!(f, "while ")?;

				condition.format_human_readable(f, false, 0)?;

				write!(f, " ")?;

				body.format_human_readable(f, true, tab_level)
			},
			ASTNode::Binary { left, op, right } =>
				write!(f, "({left} {op} {right})"),
			ASTNode::Unary {op, value} =>
				write!(f, "{op}({value})"),
			ASTNode::Function {name, args} => {
				write!(f, "{name}(")?;

				for (i, arg) in args.iter().enumerate() {
					if i > 0 {
						write!(f, ", ")?;
					}

					write!(f, "{arg}")?;
				}

				write!(f, ")")
			},
			ASTNode::FunctionDefinition { name: Some(name), args, body, is_global } => {
				if *is_global {
					write!(f, "global def {name}(")?;
				} else {
					write!(f, "def {name}(")?;
				}

				for (i, arg) in args.iter().enumerate() {
					if i > 0 {
						write!(f, ", ")?;
					}

					write!(f, "{arg}")?;
				}

				write!(f, ")")?;

				body.format_human_readable(f, true, tab_level)?;

				Ok(())
			},
			ASTNode::FunctionDefinition { name: None, args, body: _, is_global: false } => {
				write!(f, "anonymous def(")?;

				for (i, arg) in args.iter().enumerate() {
					if i > 0 {
						write!(f, ", ")?;
					}

					write!(f, "{arg}")?;
				}

				Ok(())
			},
			ASTNode::FunctionDefinition { name: None, args: _, body: _, is_global: true } => {
				unreachable!("invalid combination of is_global for anonymous function")
			},
		}
	}
}
