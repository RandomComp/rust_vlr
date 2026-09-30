use std::{fmt, io::self, process};

use rustyline::{DefaultEditor, error::ReadlineError::self};

mod lexer;
mod parser;
mod interpreter;

use crate::lexer::{Lexer, LexerError, Token};
use crate::parser::{Parser, ParserError};
use crate::interpreter::{Evaluator, EvaluatorError, EvaluatorResult, EvaluatorResultWrapper};

#[derive(argh::FromArgs, Debug)]
/// An programming language made by RDevel in Rust
struct Args {
	#[argh(option, description = "program passed in as string", short = 'c')]
	cmd: Option<String>,
	#[argh(positional)]
	file: Option<String>
}

#[derive(thiserror::Error, Debug)]
enum AppError {
	#[error("Lexer error: {0}")]
	LexerError(#[from] LexerError),
	#[error("Parser error: {0}")]
	ParserError(#[from] ParserError),
	#[error("Evaluator error: {0}")]
	EvaluatorError(#[from] EvaluatorError),
	#[error("IO error: {0}")]
	IOError(#[from] io::Error),
	#[error("Fmt error: {0}")]
	FmtError(#[from] fmt::Error),
	#[error("Expected file or code from -c")]
	ExpectedFileOrCode,
}

fn run(code: &str) {
	// println!("Tokenizing expr '{}'", code);

	let mut lexer = Lexer::new(code.chars());

	let tokens: Vec<Token> = lexer.tokenize_loop().unwrap_or_else(|e| {
		eprintln!("Lexer error: {}", e);

		process::exit(1);
	});

	// for token in &tokens {
	// 	println!("token = {}", token);
	// }

	let mut parser = Parser::new(tokens);

	let ast = parser.parse_instructions().unwrap_or_else(|e| {
		eprintln!("Parser error: {}", e);

		process::exit(1);
	}).unwrap();

	// println!("ast = {}", ast);

	let mut evaluator = Evaluator::new();
	evaluator.init_builtin_funcs();

	evaluator.eval(Box::new(ast)).unwrap_or_else(|e| {
		eprintln!("Evaluator error: {}", e);

		process::exit(1);
	});

	// println!("result = {}", result);
}

fn repl() -> Result<(), AppError> {
	let mut rl = DefaultEditor::new().unwrap();

	let mut evaluator = Evaluator::new();
	evaluator.init_builtin_funcs();

	loop {
		// println!("Tokenizing expr '{}'", code);

		let code = match rl.readline(">>> ") {
			Ok(code) => code,
			Err(ReadlineError::Eof) => break Ok(()),
			Err(ReadlineError::Interrupted) => break Ok(()),
			Err(e) => {
				eprintln!("Readline error: {}", e);

				break Ok(());
			}
		};

		rl.add_history_entry(code.clone()).unwrap();

		let mut lexer = Lexer::new(code.chars());

		let tokens: Vec<Token> = match lexer.tokenize_loop() {
			Ok(x) => x,
			Err(e) => {
				eprintln!("Lexer error: {}", e);

				continue;
			}
		};

		// for token in &tokens {
		// 	println!("token = {}", token);
		// }

		let mut parser = Parser::new(tokens);

		let ast = match parser.parse_instructions() {
			Ok(x) => x.unwrap(),
			Err(e) => {
				eprintln!("Parser error: {}", e);

				continue;
			}
		};

		// println!("ast = {}", ast);

		match evaluator.eval(Box::new(ast)) {
			Ok(EvaluatorResultWrapper { value: EvaluatorResult::None, column: _, row: _ }) => {},
			Ok(EvaluatorResultWrapper { value, column: _, row: _ }) => {
				let mut str = String::new();

				value.to_str(&mut str)?;

				println!("{}", str)
			},
			Err(e) => {
				eprintln!("Evaluator error: {}", e);

				continue;
			},
		};
	}
}

fn main()  {
	let args: Args = argh::from_env();

	match args {
		Args { cmd: None, file: Some(file) } => {
			let code = std::fs::read_to_string(file).unwrap_or_else(|e| {
				eprintln!("{}", e);

				process::exit(e.raw_os_error().unwrap());
			});

			run(&code)
		},
		Args { cmd: Some(cmd), file: None } => run(&cmd),
		Args { cmd: None, file: None } => repl().unwrap_or_else(|e| {
			eprintln!("{}", e);

			process::exit(1);
		}),
		Args { cmd: Some(_), file: Some(_) } => {
			eprintln!("{}", AppError::ExpectedFileOrCode);
		},
	}
}
