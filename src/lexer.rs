// src/lexer.rs

#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    Word(String),
    Pipe,        // |
    Less,        // <
    Greater,     // >
}

pub fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();
    let mut current_word = String::new();
    let mut in_quote: Option<char> = None;

    while let Some(ch) = chars.next() {
        match in_quote {
            Some(quote_char) => {
                if ch == quote_char {
                    in_quote = None;
                } else {
                    current_word.push(ch);
                }
            }
            None => {
                match ch {
                    '\'' | '"' => {
                        in_quote = Some(ch);
                    }
                    ' ' | '\t' | '\n' | '\r' => {
                        if !current_word.is_empty() {
                            tokens.push(Token::Word(current_word));
                            current_word = String::new();
                        }
                    }
                    '|' | '<' | '>' => {
                        if !current_word.is_empty() {
                            tokens.push(Token::Word(current_word));
                            current_word = String::new();
                        }
                        match ch {
                            '|' => tokens.push(Token::Pipe),
                            '<' => tokens.push(Token::Less),
                            '>' => tokens.push(Token::Greater),
                            _ => unreachable!(),
                        }
                    }
                    _ => {
                        current_word.push(ch);
                    }
                }
            }
        }
    }

    if in_quote.is_some() {
        return Err("Syntax error: Unterminated quote".to_string());
    }

    if !current_word.is_empty() {
        tokens.push(Token::Word(current_word));
    }

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_whitespace_splitting() {
        let input = "echo one two   three";
        let expected = vec![
            Token::Word("echo".to_string()),
            Token::Word("one".to_string()),
            Token::Word("two".to_string()),
            Token::Word("three".to_string()),
        ];
        assert_eq!(tokenize(input).unwrap(), expected);
    }

    #[test]
    fn test_quoting_behavior() {
        let input = "echo \"a  b\" 'c  d'";
        let expected = vec![
            Token::Word("echo".to_string()),
            Token::Word("a  b".to_string()),
            Token::Word("c  d".to_string()),
        ];
        assert_eq!(tokenize(input).unwrap(), expected);
    }
}