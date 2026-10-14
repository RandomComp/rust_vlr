#![warn(clippy::pedantic)]
// #![warn(clippy::restriction)]

#![deny(clippy::missing_errors_doc)]
// #![deny(clippy::missing_docs_in_private_items)]

use std::fmt::{Debug, Display};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::ExitCode;
use std::{fmt, io::self, process};

use rustyline::{DefaultEditor, error::ReadlineError::self};

mod native_func;
mod wrapper;
mod uni_type;
mod bytecode;
mod compiler;
mod vm;

use crate::compiler::{Compiler, lexer::Lexer, parser::Parser, optimizer::Optimizer};
use crate::bytecode::{Bytecode, BytecodeError};
use crate::vm::{VM, VMError};
use crate::wrapper::{LexerError, ParserError, OptimizerError, CompilerError};

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

#[derive(Debug, thiserror::Error)]
enum AppError {
	Lexer(#[from] LexerError),
	Parser(#[from] ParserError),
	Optimizer(#[from] OptimizerError),
	Compiler(#[from] CompilerError),
	Bytecode(#[from] BytecodeError),
	VM(VMError),
	IO(#[from] io::Error),
	Fmt(#[from] fmt::Error),
	Readline(#[from] ReadlineError),
}

impl From<VMError> for AppError {
	fn from(value: VMError) -> Self {
		AppError::VM(value)
	}
}

impl Display for AppError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Lexer(e) => {
				write!(f, "Tokenizing error: {e}")
			}
			Self::Parser(e) => {
				write!(f, "Parsing error: {e}")
			}
			Self::Optimizer(e) => {
				write!(f, "Optimizing error: {e}")
			}
			Self::Compiler(e) => {
				write!(f, "Compiler error: {e}")
			}
			Self::Bytecode(e) => {
				write!(f, "Assembling/Dissasembling error: {e}")
			}
			Self::VM(e) => {
				write!(f, "VM error: {e}")
			}
			Self::IO(e) => {
				write!(f, "IO error: {e}")
			}
			Self::Fmt(e) => {
				write!(f, "Fmt error: {e}")
			}
			Self::Readline(e) => {
				write!(f, "Readline error: {e}")
			}
		}
	}
}

fn format(e: &AppError, spans: &[(usize, usize)]) -> String {
	match e {
		AppError::Lexer(e) => {
			let (row, column) = spans.get(e.span).unwrap_or_else(|| panic!("invalid span present {}", e.span));

			format!("line {} at {}: Tokenizing error: {e}", row + 1, column + 1)
		}
		AppError::Parser(e) => {
			let (row, column) = spans.get(e.span).unwrap_or_else(|| panic!("invalid span present {}", e.span));

			format!("line {} at {}: Parsing error: {e}", row + 1, column + 1)
		}
		AppError::Optimizer(e) => {
			let (row, column) = spans.get(e.span).unwrap_or_else(|| panic!("invalid span present {}", e.span));

			format!("line {} at {}: Optimizing error: {e}", row + 1, column + 1)
		}
		AppError::Compiler(e) => {
			let (row, column) = spans.get(e.span).unwrap_or_else(|| panic!("invalid span present {}", e.span));

			format!("line {} at {}: Compiler error: {e}", row + 1, column + 1)
		}
		AppError::Bytecode(e) => {
			// let (row, column) = spans.get(e.span).expect(&format!("invalid span present {}", e.span));

			// format!("line {} at {}: Assembling error: {e}", row + 1, column + 1)

			format!("Assembling/Dissasembling error: {e}")
		}
		AppError::VM(e) => {
			// let (row, column) = spans.get(e.span).expect(&format!("invalid span present {}", e.span));

			// format!("line {} at {}: VM error: {e}", row + 1, column + 1)

			format!("VM error: {e}")
		}
		AppError::IO(e) => {
			// let (row, column) = spans.get(e.span).expect(&format!("invalid span present {}", e.span));

			// format!("line {} at {}: IO error: {e}", row + 1, column + 1)

			format!("IO error: {e}")
		}
		AppError::Fmt(e) => {
			// let (row, column) = spans.get(e.span).expect(&format!("invalid span present {}", e.span));

			// format!("line {} at {}: Fmt error: {e}", row + 1, column + 1)

			format!("Fmt error: {e}")
		}
		AppError::Readline(e) => {
			// let (row, column) = spans.get(e.span).expect(&format!("invalid span present {}", e.span));

			// format!("line {} at {}: Readline error: {e}", row + 1, column + 1)

			format!("Readline error: {e}")
		}
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

		let mut spans: Vec<(usize, usize)> = Vec::new();

		let mut lexer = Lexer::new(&mut spans, code.chars());

		let tokens = match lexer.tokenize_loop() {
			Ok(x) => x,
			Err(e) => {
				eprintln!("Lexer error: {e}");

				continue;
			}
		};

		for token in &tokens {
			println!("token = {token}");
		}

		let mut parser = Parser::new(tokens);

		let ast = match parser.parse_instructions() {
			Ok(x) => x,
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

fn format_code(code: &str) -> Result<(), AppError> {
	// println!("Tokenizing expr '{}'", code);

	let mut spans = Vec::new();

	let mut lexer = Lexer::new(&mut spans, code.chars());

	let tokens = lexer.tokenize_loop().unwrap_or_else(|e| {
		eprintln!("Lexer error: {e}");

		process::exit(1);
	});

	for token in &tokens {
		println!("token = {token}");
	}

	let mut parser = Parser::new(tokens);

	let mut ast = parser.parse_instructions()?;

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

type Span = Vec<(usize, usize)>;

fn compile(code: &str) -> Result<Vec<u8>, (AppError, Span)> {
	// println!("Tokenizing expr '{}'", code);

	let mut spans = Vec::new();

	let mut lexer = Lexer::new(&mut spans, code.chars());

	let tokens = match lexer.tokenize_loop() {
		Ok(res) => res,
		Err(e) => {
			return Err((e.into(), spans))
		}
	};

	for token in &tokens {
		println!("token = {token}");
	}

	let mut parser = Parser::new(tokens);

	let mut ast = match parser.parse_instructions() {
		Ok(res) => res,
		Err(e) => {
			return Err((e.into(), spans))
		}
	};

	let mut ast_str = String::new();

	if let Err(e) = ast.format_human_readable(&mut ast_str, false, 0) {
		return Err((e.into(), spans))
	}

	println!("ast = {ast}");
	println!("ast = {ast_str}");

// 	let mut optimizer = Optimizer::new();
//
// 	if let Err(e) = optimizer.fold(&mut ast) {
// 		return Err((e.into(), spans))
// 	}
//
// 	ast_str.clear();
//
// 	if let Err(e) = ast.format_human_readable(&mut ast_str, false, 0) {
// 		return Err((e.into(), spans))
// 	}
//
// 	println!("ast after optimizer = {ast_str}");
//
// 	for var in optimizer.unused_vars {
// 		println!("'{var}' unused");
// 	}

	let mut compiler = Compiler::new();

	let mut bytes = Vec::new();

	if let Err(e) = compiler.compile_loop(&ast, &mut bytes) {
		return Err((e.into(), spans))
	}

	for (i, byte) in bytes.iter().enumerate() {
		println!("{i:02}: {byte}");
	}

	let bytes = match Bytecode::asm(bytes) {
		Ok(res) => res,
		Err(e) => {
			return Err((e.into(), spans))
		}
	};

	for byte in &bytes {
		print!("{byte:02X} ");
	}

	println!();

	Ok(bytes)
}

fn format_file(file: &str) -> Result<(), AppError> {
	let code = std::fs::read_to_string(file)?;

	format_code(&code)
}

fn compile_file(file: &str) -> Result<(), (AppError, Option<Span>)> {
	let code = std::fs::read_to_string(file).map_err(|e| (e.into(), None))?;

	let mut out_file = PathBuf::from(file);

	out_file.set_extension("out");

	let bytes = compile(&code).map_err(|(e, spans)| (e, Some(spans)))?;

	let mut file = std::fs::File::create(out_file).map_err(|e| (e.into(), None))?;

	writeln!(file, "#!{} -r", std::env::current_exe().map_err(|e| (e.into(), None))?.display()).map_err(|e| (e.into(), None))?;
	file.write_all(&bytes).map_err(|e| (e.into(), None))?;

	let mut perms = file.metadata().map_err(|e| (e.into(), None))?.permissions();
	perms.set_mode(0o744);
	file.set_permissions(perms).map_err(|e| (e.into(), None))?;

	Ok(())
}

fn app(args: Args) -> Result<(), AppError> {
	let err = match args {
		Args { format: Some(format), .. } =>
			format_file(&format),
		Args { run: None, compile: Some(file), .. } => {
			if let Err((e, spans)) = compile_file(&file) {
				if let Some(spans) = spans {
					eprintln!("{}", format(&e, &spans));
				} else {
					eprintln!("{e}");
				}

				return Err(e)
			}

			return Ok(())
		},
		Args { run: Some(file_to_run), compile: Some(file_to_compile), .. } => {
			if let Err((e, spans)) = compile_file(&file_to_compile) {
				if let Some(spans) = spans {
					eprintln!("{}", format(&e, &spans));
				} else {
					eprintln!("{e}");
				}

				return Err(e)
			}

			run_file(&file_to_run)
		},
		Args { run: Some(file), compile: None, .. } =>
			run_file(&file),
		Args { run: None, compile: None, .. } =>
			repl(),
	};

	if let Err(e) = &err {
		eprintln!("{e}");
	}

	err
}

fn main() -> ExitCode {
	let args: Args = argh::from_env();

	let res = app(args);

	if res.is_err() {
		return ExitCode::FAILURE
	}

	ExitCode::SUCCESS
}
