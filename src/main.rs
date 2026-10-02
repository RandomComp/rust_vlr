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
mod compiler;
mod vm;

use crate::vm::{VM, VMError};
use crate::compiler::{Compiler, CompilerError};
use crate::lexer::{Lexer, LexerError, Token};
use crate::parser::{Parser, ParserError};

#[derive(argh::FromArgs, Debug)]
/// An programming language made by RDevel in Rust
struct Args {
	#[argh(option, description = "run bytecode file", short = 'r')]
	run: Option<String>,
	#[argh(option, description = "file to compile", short = 'c')]
	file_to_compile: Option<String>
}

#[derive(thiserror::Error, Debug)]
enum AppError {
	#[error("Lexer error: {0}")]
	Lexer(#[from] LexerError),
	#[error("Parser error: {0}")]
	Parser(#[from] ParserError),
	#[error("Compiler error: {0}")]
	Compiler(#[from] CompilerError),
	#[error("VM error: {0}")]
	VM(#[from] VMError),
	#[error("IO error: {0}")]
	IO(#[from] io::Error),
	#[error("Fmt error: {0}")]
	Fmt(#[from] fmt::Error),
}

fn repl() -> Result<(), AppError> {
	let mut rl = DefaultEditor::new().unwrap();

	// let mut compiler = Compiler::new();

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

		rl.add_history_entry(code.clone()).unwrap();

		let mut lexer = Lexer::new(code.chars());

		let tokens: Vec<Token> = match lexer.tokenize_loop() {
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
			Ok(x) => x.unwrap(),
			Err(e) => {
				eprintln!("Parser error: {e}");

				continue;
			}
		};

		println!("ast = {ast}");

		let mut result = Vec::new();

		if let Err(e) = Compiler::compile(&ast, &mut result) {
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
	}
}

fn run_file(file: &str) -> Result<(), AppError> {
	let bytecode = std::fs::read(file).unwrap_or_else(|e| {
		eprintln!("{e}");

		process::exit(e.raw_os_error().unwrap());
	});

	let mut start_index = 0;

	if bytecode.starts_with(b"#!") {
		start_index = bytecode.iter().position(|&c| c == b'\n').unwrap() + 1;
	}

	let mut interpreter = VM::new(Some((&bytecode[start_index..]).into()));

	interpreter.disasm()?;

	if let Some(result) = interpreter.exec()? {
		println!("result = {result}");
	}

	Ok(())
}

fn compile(code: &str) -> Result<Vec<u8>, CompilerError> {
	// println!("Tokenizing expr '{}'", code);

	let mut lexer = Lexer::new(code.chars());

	let tokens: Vec<Token> = lexer.tokenize_loop().unwrap_or_else(|e| {
		eprintln!("Lexer error: {e}");

		process::exit(1);
	});

	// for token in &tokens {
	// 	println!("token = {}", token);
	// }

	let mut parser = Parser::new(tokens);

	let ast = parser.parse_instructions().unwrap_or_else(|e| {
		eprintln!("Parser error: {e}");

		process::exit(1);
	}).unwrap();

	println!("ast = {ast}");

	// let mut compiler = Compiler::new();

	let mut bytes = Vec::new();

	Compiler::compile(&ast, &mut bytes)?;

	for byte in &bytes {
		print!("{byte:02X} ");
	}

	println!();

	Ok(bytes)
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

fn main() {
	let args: Args = argh::from_env();

	match args {
		Args { run: Some(file_to_run), file_to_compile: Some(file_to_compile) } => {
			compile_file(&file_to_compile).unwrap_or_else(|e| {
				eprintln!("{e}");

				process::exit(1);
			});
			run_file(&file_to_run).unwrap_or_else(|e| {
				eprintln!("{e}");

				process::exit(1);
			});
		},
		Args { run: Some(file), file_to_compile: None } =>
			run_file(&file).unwrap_or_else(|e| {
				eprintln!("{e}");

				process::exit(1);
			}),
		Args { run: None, file_to_compile: Some(file) } =>
			compile_file(&file).unwrap_or_else(|e| {
				eprintln!("{e}");

				process::exit(1);
			}),
		Args { run: None, file_to_compile: None } => {
			repl().unwrap_or_else(|e| {
				eprintln!("{e}");

				process::exit(1);
			});
		}
	}
}
