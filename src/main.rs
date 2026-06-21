// src/main.rs
mod lexer;
mod parser;
mod exec;

use std::io::{self, Write};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Process non-interactive execution requests (-c)
    if args.len() > 1 && args[1] == "-c" {
        if args.len() < 3 {
            eprintln!("minish: -c requires an argument");
            std::process::exit(1);
        }
        let command_str = &args[2];
        let exit_code = run_line(command_str);
        std::process::exit(exit_code);
    }

    // Process standard interactive REPL layout
    loop {
        print!("minish> ");
        if let Err(e) = io::stdout().flush() {
            eprintln!("minish: flushing error: {}", e);
            continue;
        }

        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(0) => {
                println!();
                break; // Gracefully handle EOF (Ctrl+D)
            }
            Ok(_) => {
                let trimmed = input.trim();
                if trimmed.is_empty() {
                    continue;
                }
                run_line(trimmed);
            }
            Err(e) => {
                eprintln!("minish: read error: {}", e);
            }
        }
    }
}

fn run_line(line: &str) -> i32 {
    let tokens = match lexer::tokenize(line) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{}", e);
            return 1;
        }
    };

    let pipeline = match parser::parse(tokens) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}", e);
            return 1;
        }
    };

    exec::execute_pipeline(&pipeline)
}