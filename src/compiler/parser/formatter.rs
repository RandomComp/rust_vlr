use std::fmt::Write;

use crate::compiler::parser::RawASTNode;

impl std::fmt::Display for RawASTNode {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			RawASTNode::None => write!(f, "()"),
			RawASTNode::Boolean(value) => write!(f, "{value}"),
			RawASTNode::Float(value) => write!(f, "{value}"),
			RawASTNode::String(text) => write!(f, "\"{text}\""),
			RawASTNode::Variable(name) => write!(f, "{name}"),
			RawASTNode::List(values) => {
				write!(f, "[")?;

				for (i, value) in values.iter().enumerate() {
					if i > 0 {
						write!(f, ", ")?;
					}

					write!(f, "{value}")?;
				}

				write!(f, "]")
			},
			RawASTNode::Block(values) => {
				write!(f, "{{ ")?;

				for value in values {
					write!(f, "{value}; ")?;
				}

				write!(f, "}}")?;

				Ok(())
			},
			RawASTNode::Function {name, args} => {
				write!(f, "{name}(")?;

				for (i, arg) in args.iter().enumerate() {
					if i > 0 {
						write!(f, ", ")?;
					}

					write!(f, "{arg}")?;
				}

				write!(f, ")")
			},
			RawASTNode::If { condition, body, else_body: Some(else_body) } => {
				write!(f, "if {condition} {body} else {else_body}")
			},
			RawASTNode::If { condition, body, else_body: None } => {
				write!(f, "if {condition} {body}")
			},
			RawASTNode::Break(value) => {
				write!(f, "break {value}")
			},
			RawASTNode::Return(value) => {
				write!(f, "return {value}")
			},
			RawASTNode::While { condition, body, else_body: Some(else_body) } => {
				write!(f, "while {condition} {body} else {else_body}")
			},
			RawASTNode::While { condition, body, else_body: None } => {
				write!(f, "while {condition} {body}")
			},
			// ASTNodeEnum::Ternary { left, op, center, right } =>
			// 	write!(f, "({} {} {} {} {})", left, op, center, op, right),
			RawASTNode::Binary { left, op, right } =>
				write!(f, "({left} {op} {right})"),
			RawASTNode::Unary {op, value} =>
				write!(f, "{op}({value})"),
			RawASTNode::FunctionDefinition { name: Some(name), args, body } => {
				write!(f, "def {name}(")?;

				for (i, arg) in args.iter().enumerate() {
					if i > 0 {
						write!(f, ", ")?;
					}

					write!(f, "{arg}")?;
				}

				write!(f, ") {body}")?;

				Ok(())
			},
			RawASTNode::FunctionDefinition { name: None, args, .. } => {
				write!(f, "anonymous def(")?;

				for (i, arg) in args.iter().enumerate() {
					if i > 0 {
						write!(f, ", ")?;
					}

					write!(f, "{arg}")?;
				}

				Ok(())
			},
		}
	}
}

impl RawASTNode {
	pub fn format_human_readable(&self, f: &mut String, for_statement: bool, tab_level: usize) -> std::fmt::Result {
		let tab_level_syms: String = std::iter::repeat_n(' ', 4 * tab_level).collect();

		if !for_statement {
			write!(f, "{tab_level_syms}")?;
		}

		match self {
			RawASTNode::None => write!(f, "()"),
			RawASTNode::Boolean(value) => write!(f, "{value}"),
			RawASTNode::Float(value) => write!(f, "{value}"),
			RawASTNode::String(text) => write!(f, "\"{text}\""),
			RawASTNode::Variable(name) => write!(f, "{name}"),
			RawASTNode::List(values) => {
				write!(f, "[")?;

				for (i, value) in values.iter().enumerate() {
					if i > 0 {
						write!(f, ", ")?;
					}

					value.format_human_readable(f, false, 0)?;
				}

				write!(f, "]")
			},
			RawASTNode::Block(values) => {
				writeln!(f, "{{")?;

				for value in values {
					value.format_human_readable(f, false, tab_level + 1)?;

					writeln!(f, ";")?;
				}

				write!(f, "{tab_level_syms}}}")
			},
			RawASTNode::If { condition, body, else_body: Some(else_body) } => {
				write!(f, "if ")?;

				condition.format_human_readable(f, false, 0)?;

				write!(f, " ")?;

				body.format_human_readable(f, true, tab_level + 1)?;

				write!(f, " ")?;

				write!(f, "else ")?;

				write!(f, " ")?;

				else_body.format_human_readable(f, true, tab_level + 1)
			},
			RawASTNode::If { condition, body, else_body: None } => {
				write!(f, "if ")?;

				condition.format_human_readable(f, false, 0)?;

				write!(f, " ")?;

				body.format_human_readable(f, true, tab_level)
			},
			RawASTNode::Break(value) => {
				write!(f, "break {value}")
			},
			RawASTNode::Return(value) => {
				write!(f, "return {value}")
			},
			RawASTNode::While { condition, body, else_body: Some(else_body) } => {
				write!(f, "while ")?;

				condition.format_human_readable(f, false, 0)?;

				write!(f, " ")?;

				body.format_human_readable(f, false, tab_level + 1)?;

				write!(f, " ")?;

				write!(f, "else ")?;

				write!(f, " ")?;

				else_body.format_human_readable(f, true, tab_level + 1)
			},
			RawASTNode::While { condition, body, else_body: None } => {
				write!(f, "while ")?;

				condition.format_human_readable(f, false, 0)?;

				write!(f, " ")?;

				body.format_human_readable(f, true, tab_level)
			},
			RawASTNode::Binary { left, op, right } =>
				write!(f, "({left} {op} {right})"),
			RawASTNode::Unary {op, value} =>
				write!(f, "{op}({value})"),
			RawASTNode::Function {name, args} => {
				write!(f, "{name}(")?;

				for (i, arg) in args.iter().enumerate() {
					if i > 0 {
						write!(f, ", ")?;
					}

					write!(f, "{arg}")?;
				}

				write!(f, ")")
			},
			RawASTNode::FunctionDefinition { name: Some(name), args, body } => {
				write!(f, "def {name}(")?;

				for (i, arg) in args.iter().enumerate() {
					if i > 0 {
						write!(f, ", ")?;
					}

					write!(f, "{arg}")?;
				}

				write!(f, ") ")?;

				body.format_human_readable(f, true, tab_level)?;

				Ok(())
			},
			RawASTNode::FunctionDefinition { name: None, args, body } => {
				write!(f, "anonymous def(")?;

				for (i, arg) in args.iter().enumerate() {
					if i > 0 {
						write!(f, ", ")?;
					}

					write!(f, "{arg}")?;
				}

				write!(f, ") ")?;

				body.format_human_readable(f, true, tab_level)
			},
		}
	}
}
