#![warn(clippy::pedantic)]
// #![warn(clippy::restriction)]

#![deny(clippy::missing_errors_doc)]
// #![deny(clippy::missing_docs_in_private_items)]

use std::fmt::{Write, Debug, Display};
use std::io::Write as IoWrite;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::ExitCode;
use std::{fmt, io::self, process};

use rustyline::{DefaultEditor, error::ReadlineError::self};

mod wrapper;
mod assembler;
mod compiler;
mod vm;

use crate::assembler::defs::BytecodeError::{self, NativeFunctionID};
use crate::assembler::native_func::NativeFunctionIDError;
use crate::assembler::optimizer::AssemblerOptimizer;
use crate::compiler::CompilerError;
use crate::compiler::lexer::LexerError;
use crate::compiler::optimizer::OptimizerError;
use crate::compiler::parser::ParserError;
use crate::compiler::{Compiler, lexer::Lexer, parser::Parser, optimizer::Optimizer};
use crate::assembler::{Bytecode};
use crate::vm::{RawVMError, VM, VMError};
use crate::wrapper::Spanned;

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
	Spanned(Box<Spanned<AppError>>)
}

impl<T> From<Spanned<T>> for AppError where AppError: From<T>  {
	fn from(value: Spanned<T>) -> Self {
		let span = value.span;

		AppError::Spanned(Box::new(
			Spanned::from_with_span(
				value.val.into(),
				span
			)
		))
	}
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
			Self::Spanned(spanned) => {
				write!(f, "{}", spanned.val)
			}
		}
	}
}

fn format(spans: Option<&[(usize, usize)]>, e: &AppError, f: &mut String) -> fmt::Result {
	match e {
		AppError::Spanned(spanned) if let Some(spans) = spans => {
			write!(f, "{}", spanned.format(spans))
		},
		_ => write!(f, "{e}")
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

		if let Ok(result) = result {
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

	let result = interpreter.exec()?;

	println!("result = {result}");

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

type Spans = Vec<(usize, usize)>;

fn compile(code: &str) -> Result<Vec<u8>, (AppError, Spans)> {
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

	let mut optimizer = Optimizer::new();

	if let Err(e) = optimizer.fold(&mut ast) {
		return Err((e.into(), spans))
	}

	ast_str.clear();

	if let Err(e) = ast.format_human_readable(&mut ast_str, false, 0) {
		return Err((e.into(), spans))
	}

	println!("ast after optimizer = {ast_str}");

	for var in optimizer.unused_vars {
		println!("'{var}' unused");
	}

	let mut compiler = Compiler::new();

	let mut bytes = Vec::new();

	if let Err(e) = compiler.compile_loop(&ast, &mut bytes) {
		return Err((e.into(), spans))
	}

	println!("Bytecode before optimizer:");

	for (i, byte) in bytes.iter().enumerate() {
		println!("{i:02}: {byte}");
	}

	let mut asm_optimizer = AssemblerOptimizer::new(&bytes);

	let optimized = asm_optimizer.optimize();

	println!("Bytecode after optimizer:");

	for (i, byte) in optimized.iter().enumerate() {
		println!("{i:02}: {byte}");
	}

	let bytes = match Bytecode::asm(optimized) {
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

fn compile_file(file: &str) -> Result<(), (AppError, Option<Spans>)> {
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
				let mut e_str = String::new();
				format(spans.as_deref(), &e, &mut e_str)?;

				eprintln!("{e_str}");

				return Err(e)
			}

			return Ok(())
		},
		Args { run: Some(file_to_run), compile: Some(file_to_compile), .. } => {
			if let Err((e, spans)) = compile_file(&file_to_compile) {
				let mut e_str = String::new();
				format(spans.as_deref(), &e, &mut e_str)?;

				eprintln!("{e_str}");

				return Err(e)
			}

			run_file(&file_to_run)
		},
		Args { run: Some(file), compile: None, .. } =>
			run_file(&file),
		Args { run: None, compile: None, .. } =>
			repl(),
	};

	if let Err(AppError::VM(VMError {val: RawVMError::NativeFunction( NativeFunctionIDError::Exit(_) ), ..})) = &err {
		return err
	}

	if let Err(e) = &err {
		eprintln!("{e}");
	}

	err
}

fn main() -> ExitCode {
	let args: Args = argh::from_env();

	let res = app(args);

	if let Err(AppError::VM(VMError {val: RawVMError::NativeFunction( NativeFunctionIDError::Exit(code) ), ..})) = &res {
		return ExitCode::from(
			(*code & 0xFF) as u8
		)
	}

	if res.is_err() {
		return ExitCode::FAILURE
	}

	ExitCode::SUCCESS
}
