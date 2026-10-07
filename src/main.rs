#![warn(clippy::pedantic)]

#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_docs_in_private_items)]

use std::fmt::Debug;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::{fmt, io::self, process};

use rustyline::{DefaultEditor, error::ReadlineError::self};

mod lexer;
mod parser;
mod optimizer;
mod uni_type;
mod bytecode;
mod compiler;
mod vm;

use crate::optimizer::{Optimizer, OptimizerError};
use crate::vm::{VM, VMError};
use crate::bytecode::{Bytecode, BytecodeError};
use crate::compiler::{Compiler, CompilerError};
use crate::lexer::{Lexer, LexerError};
use crate::parser::{Parser, ParserError};

#[derive(argh::FromArgs, Debug)]
#[argh(description="An programming language made by RDevel in Rust")]
struct Args {
	#[argh(option, description = "run bytecode file", short = 'r')]
	run: Option<String>,
	#[argh(option, description = "file to compile", short = 'c')]
	compile: Option<String>,
	#[argh(option, description = "file to format", short = 'f')]
	format: Option<String>,
}

#[derive(thiserror::Error)]
enum AppError {
	#[error("Lexer error: {0}")]
	Lexer(#[from] LexerError),
	#[error("Parser error: {0}")]
	Parser(#[from] ParserError),
	#[error("Optimizer error: {0}")]
	Optimizer(#[from] OptimizerError),
	#[error("Compiler error: {0}")]
	Compiler(#[from] CompilerError),
	#[error("Bytecode error: {0}")]
	Bytecode(#[from] BytecodeError),
	#[error("VM error: {0}")]
	VM(#[from] VMError),
	#[error("IO error: {0}")]
	IO(#[from] io::Error),
	#[error("Fmt error: {0}")]
	Fmt(#[from] fmt::Error),
	#[error("Readline error: {0}")]
	Readline(#[from] ReadlineError),
	#[error("Empty code")]
	EmptyCode,
}

impl Debug for AppError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    	write!(f, "{self}")
	}
}

fn repl() -> Result<(), AppError> {
	let mut rl = DefaultEditor::new()?;

	let mut vm = VM::new(None);

	loop {
		// println!("Tokenizing expr '{}'", code);

		let code = match rl.readline(">>> ") {
			Ok(code) => code,
			Err(ReadlineError::Eof | ReadlineError::Interrupted) => break Ok(()),
			Err(e) => {
				eprintln!("Readline error: {e}");

				break Ok(());
			}
		};

		let mut lexer = Lexer::new(code.chars());

		let tokens = match lexer.tokenize_loop() {
			Ok(x) => x,
			Err(e) => {
				eprintln!("Lexer error: {e}");

				continue;
			}
		};

		// for token in &tokens {
		// 	println!("token = {token}");
		// }

		let mut parser = Parser::new(tokens);

		let ast = match parser.parse_instructions() {
			Ok(Some(x)) => x,
			Ok(None) => continue,
			Err(e) => {
				eprintln!("Parser error: {e}");

				continue;
			}
		};

		println!("ast = {ast}");

		let mut compiler = Compiler::new();

		let mut result = Vec::new();

		if let Err(e) = compiler.compile_loop(&ast, &mut result) {
			eprintln!("Compiler error: {e}");

			continue;
		}

		vm.bytecode = Some(result);

		let result = vm.exec();

		if let Ok(Some(result)) = result {
			println!("{result}");
		} else if let Err(e) = result {
			eprintln!("VM error: {e}");
		}

		rl.add_history_entry(code)?;
	}
}

fn run_file(file: &str) -> Result<(), AppError> {
	let bytes = std::fs::read(file)?;

	let mut start_index = 0;

	if bytes.starts_with(b"#!") && let Some(shabang_pos) = bytes.iter().position(|&c| c == b'\n') {
		start_index = shabang_pos + 1;
	}

	let instructions = Bytecode::disasm(bytes.iter().skip(start_index).copied())?;

	let mut interpreter = VM::new(Some(instructions));

	if let Some(result) = interpreter.exec()? {
		println!("result = {result}");
	}

	Ok(())
}

fn format(code: &str) -> Result<(), AppError> {
	// println!("Tokenizing expr '{}'", code);

	let mut lexer = Lexer::new(code.chars());

	let tokens = lexer.tokenize_loop().unwrap_or_else(|e| {
		eprintln!("Lexer error: {e}");

		process::exit(1);
	});

	// for token in &tokens {
	// 	println!("token = {}", token);
	// }

	let mut parser = Parser::new(tokens);

	let Some(mut ast) = parser.parse_instructions()? else {
		return Ok(())
	};

	let mut ast_str = String::new();

	ast.format_human_readable(&mut ast_str, false, 0)?;

	println!("ast (with formatting) = {ast_str}");
	println!("ast (without formatting) = {ast}");

	let mut optimizer = Optimizer::new();

	optimizer.fold(&mut ast)?;

	ast_str.clear();

	ast.format_human_readable(&mut ast_str, false, 0)?;

	println!("ast after optimizer (with formatting) = {ast_str}");
	println!("ast after optimizer (without formatting) = {ast}");

	Ok(())
}

fn compile(code: &str) -> Result<Vec<u8>, AppError> {
	// println!("Tokenizing expr '{}'", code);

	let mut lexer = Lexer::new(code.chars());

	let tokens = lexer.tokenize_loop()?;

	// for token in &tokens {
	// 	println!("token = {}", token);
	// }

	let mut parser = Parser::new(tokens);

	let Some(mut ast) = parser.parse_instructions()? else {
		return Err(AppError::EmptyCode)
	};

	let mut ast_str = String::new();

	ast.format_human_readable(&mut ast_str, false, 0)?;

	println!("ast = {ast_str}");

	let mut optimizer = Optimizer::new();

	optimizer.fold(&mut ast)?;

	ast_str.clear();

	ast.format_human_readable(&mut ast_str, false, 0)?;

	println!("ast after optimizer = {ast_str}");

	for var in optimizer.unused_vars {
		println!("'{var}' unused");
	}

	let mut compiler = Compiler::new();

	let mut bytes = Vec::new();

	compiler.compile_loop(&ast, &mut bytes)?;

	for (i, byte) in bytes.iter().enumerate() {
		println!("{i:02}: {byte}");
	}

	let bytes = Bytecode::asm(bytes);

	Ok(bytes)
}

fn format_file(file: &str) -> Result<(), AppError> {
	let code = std::fs::read_to_string(file)?;

	format(&code)
}

fn compile_file(file: &str) -> Result<(), AppError> {
	let code = std::fs::read_to_string(file)?;

	let mut out_file = PathBuf::from(file);

	out_file.set_extension("out");

	let bytes = compile(&code)?;

	let mut file = std::fs::File::create(out_file)?;

	writeln!(file, "#!{} -r", std::env::current_exe()?.display())?;
	file.write_all(&bytes)?;

	let mut perms = file.metadata()?.permissions();
	perms.set_mode(0o744);
	file.set_permissions(perms)?;

	Ok(())
}

fn main() -> Result<(), AppError> {
	let args: Args = argh::from_env();

	match args {
		Args { format: Some(format), .. } =>
			format_file(&format),
		Args { run: Some(file_to_run), compile: Some(file_to_compile), .. } => {
			compile_file(&file_to_compile)?;
			run_file(&file_to_run)
		},
		Args { run: Some(file), compile: None, .. } =>
			run_file(&file),
		Args { run: None, compile: Some(file), .. } =>
			compile_file(&file),
		Args { run: None, compile: None, .. } =>
			repl(),
	}
}
