// src/parser.rs
use crate::lexer::Token;

#[derive(Debug, PartialEq, Clone)]
pub struct Pipeline {
    pub commands: Vec<Cmd>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Cmd {
    pub argv: Vec<String>,
    pub stdin_from: Option<String>,
    pub stdout_to: Option<String>,
}

impl Cmd {
    fn new() -> Self {
        Cmd {
            argv: Vec::new(),
            stdin_from: None,
            stdout_to: None,
        }
    }
}

pub fn parse(tokens: Vec<Token>) -> Result<Pipeline, String> {
    if tokens.is_empty() {
        return Err("Empty command".to_string());
    }

    let mut commands = Vec::new();
    let mut current_cmd = Cmd::new();
    let mut token_iter = tokens.into_iter().peekable();

    while let Some(token) = token_iter.next() {
        match token {
            Token::Word(w) => {
                current_cmd.argv.push(w);
            }
            Token::Less => {
                match token_iter.next() {
                    Some(Token::Word(filename)) => {
                        if current_cmd.stdin_from.is_some() {
                            return Err("Syntax error: Multiple stdin redirections".to_string());
                        }
                        current_cmd.stdin_from = Some(filename);
                    }
                    _ => return Err("Syntax error: Expected file after '<'".to_string()),
                }
            }
            Token::Greater => {
                match token_iter.next() {
                    Some(Token::Word(filename)) => {
                        if current_cmd.stdout_to.is_some() {
                            return Err("Syntax error: Multiple stdout redirections".to_string());
                        }
                        current_cmd.stdout_to = Some(filename);
                    }
                    _ => return Err("Syntax error: Expected file after '>'".to_string()),
                }
            }
            Token::Pipe => {
                if current_cmd.argv.is_empty() {
                    return Err("Syntax error: Empty command pipeline stage".to_string());
                }
                commands.push(current_cmd);
                current_cmd = Cmd::new();
            }
        }
    }

    if current_cmd.argv.is_empty() {
        return Err("Syntax error: Trailing pipe operator".to_string());
    }
    commands.push(current_cmd);

    Ok(Pipeline { commands })
}