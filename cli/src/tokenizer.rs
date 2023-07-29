use std::env::Args;
use crate::cli::CliError;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Text(String),
    Option(String),
}

pub(crate) fn tokenize_cli(args: &mut Args) -> Result<Vec<Token>, CliError> {
    // Here args are already parsed by the Unix shell, i.e. bash, zsh, ...
    let mut token = Vec::new();
    for arg in args {
        let mut tk = tokenize_arg(arg.as_str());
        token.append(&mut tk);
    }
    Ok(token)
}

pub(crate) fn tokenize_arg(arg: &str) -> Vec<Token> {
    let mut token = Vec::new();
    if arg.starts_with("--") {
        let long_option = &arg[2..];
        token.push(Token::Option(long_option.to_string()));
    } else if arg.starts_with("-") {
        let mut remainder = &arg[1..];
        while !remainder.is_empty() {
            let short_option = &remainder[0..1];
            remainder = &remainder[1..];
            token.push(Token::Option(short_option.to_string()));
        }
    } else {
        token.push(Token::Text(arg.to_string()));
    };
    token
}

pub(crate) fn tokenize_shell(line: &str) -> Result<Vec<Token>, CliError> {
    // Getting the all subcommand arguments as a single string here.
    let mut token: Vec<Token> = Vec::new();
    let mut item = String::new();
    let mut quoted = false;            // Backslash quoting is done inside quotes
    let mut escaped = false;
    let mut short_option = false;
    let mut long_option = false;
    for ch in line.chars() {
        if quoted {
            if escaped {
                escaped = false;
                match ch {
                    '"' => {item.push('"')},
                    't' => {item.push('\t')},
                    'n' => {item.push('\n')},
                    'r' => {item.push('\r')},
                    '\\' => {item.push('\\')},
                    ch => {return Err(CliError::InvalidEscape(ch));}
                };
            } else {
                match ch {
                    '\\' => {
                        escaped = true;
                    },
                    '"'  => {
                        // Do not yet add to tokens.
                        // Item may be continued.
                        quoted = false;
                    },
                    ch   => {
                        item.push(ch);
                    }
                }
            }
        } else {  // not quoted
            match ch {
                ' ' | '\t' | '\n' | '\r' => {
                    if long_option {
                        long_option = false;
                        if item.is_empty() {
                            // -- is not an option.
                            token.push(Token::Text(String::from("--")));
                        } else {
                            token.push(Token::Option(swap(&mut item)));
                        }
                    } else if short_option {
                        short_option = false;
                        if item.is_empty() {
                            // - is not an option
                            token.push(Token::Text(String::from("-")));
                        } else {
                            token.push(Token::Option(swap(&mut item)));
                        }
                    } else if item.is_empty() {
                        // Repeated white space
                    } else {
                        token.push(Token::Text(swap(&mut item)));
                    };
                },
                '-' if item.len() == 0 => {
                    if short_option {
                        long_option = true;
                        short_option = false;
                    } else {
                        short_option = true;
                    }; 
                },
                '"' => {
                    quoted = true;
                }
                // Backslash is handled as a normal character outside quotes.
                ch => {
                    if short_option && !item.is_empty() {
                        token.push(Token::Option(swap(&mut item)));
                    };
                    item.push(ch);
                }
            }
        }
    }
    if escaped {
        return Err(CliError::MissingEscapedCharacter);
    } else if quoted {
        return Err(CliError::MissingClosingQuote);
    } else if long_option && item.is_empty() {
        token.push(Token::Text(String::from("--")));
    } else if short_option && item.is_empty() {
        token.push(Token::Text(String::from("-")));
    } else if long_option || short_option {
        token.push(Token::Option(item));
    } else if !item.is_empty() {
        token.push(Token::Text(item));
    }
    Ok(token)
}

fn swap(value: &mut String) -> String {
    let mut other = String::new();
    std::mem::swap(value, &mut other);
    other
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splitting_on_whitespace() {
        assert_eq!(
            tokenize_shell("This text is\rsplit\ton white\nspaces. 안녕하세요 end").unwrap(),
            vec!(
                Token::Text("This".to_string()),
                Token::Text("text".to_string()),
                Token::Text("is".to_string()),
                Token::Text("split".to_string()),
                Token::Text("on".to_string()),
                Token::Text("white".to_string()),
                Token::Text("spaces.".to_string()),
                Token::Text("안녕하세요".to_string()),
                Token::Text("end".to_string()),
            )
        );
    }

    #[test]
    fn quoted_strings() {
        assert_eq!(
            tokenize_shell(r#""Peter Meier" spricht mit "Max Mustermann""#).unwrap(),
            vec!(
                Token::Text("Peter Meier".to_string()),
                Token::Text("spricht".to_string()),
                Token::Text("mit".to_string()),
                Token::Text("Max Mustermann".to_string()),
            )
        );
    }

    #[test]
    fn joined_quoted_string() {
        assert_eq!(
            tokenize_shell(r#"Herr "Max ""Mustermann""#).unwrap(),
            vec!(
                Token::Text("Herr".to_string()),
                Token::Text("Max Mustermann".to_string()),
            )
        );
    }

    #[test]
    fn escaped_quotes_outside_of_quoted_text_are_normal_characters() {
        assert_eq!(
            tokenize_shell(r#"Quotes \"outside of" quoted text."#).unwrap(),
            vec!(
                Token::Text("Quotes".to_string()),
                Token::Text("\\outside of".to_string()),
                Token::Text("quoted".to_string()),
                Token::Text("text.".to_string()),
            )
        );
    }

    #[test]
    fn escaped_quotes_inside_of_quoted_text() {
        assert_eq!(
            tokenize_shell(r#"Escaped quotes "\"inside\"\"" of quoted text."#).unwrap(),
            vec!(
                Token::Text("Escaped".to_string()),
                Token::Text("quotes".to_string()),
                Token::Text("\"inside\"\"".to_string()),
                Token::Text("of".to_string()),
                Token::Text("quoted".to_string()),
                Token::Text("text.".to_string()),
            )
        );
    }

    #[test]
    fn escaped_characters() {
        assert_eq!(
            tokenize_shell(r#"All escaped characters are: "\"\t\n\r\\" ."#).unwrap(),
            vec!(
                Token::Text("All".to_string()),
                Token::Text("escaped".to_string()),
                Token::Text("characters".to_string()),
                Token::Text("are:".to_string()),
                Token::Text("\"\t\n\r\\".to_string()),
                Token::Text(".".to_string()),
            )
        );
    }

    #[test]
    fn invalid_escaped_characters() {
        // Can't use assert_eq! here, since PartialEq is not implemented for std::io::Error.
        assert!(matches!(
            tokenize_shell(r#"Invalid: "\e""#).unwrap_err(),
            CliError::InvalidEscape('e')
        ));
    }

    #[test]
    fn missing_escaped_character() {
        assert!(matches!(
            tokenize_shell(r#""text\"#).unwrap_err(),
            CliError::MissingEscapedCharacter
        ));
    }

    #[test]
    fn missing_closing_quotes() {
        assert!(matches!(
            tokenize_shell(r#""text"#).unwrap_err(),
            CliError::MissingClosingQuote
        ));
    }

    #[test]
    fn long_options() {
        assert_eq!(
            tokenize_shell(r#"--foo --bar"#).unwrap(),
            vec!(
                Token::Option("foo".to_string()),
                Token::Option("bar".to_string()),
            )
        );
    }

    #[test]
    fn short_options() {
        assert_eq!(
            tokenize_shell(r#"-foo -bar"#).unwrap(),
            vec!(
                Token::Option("f".to_string()),
                Token::Option("o".to_string()),
                Token::Option("o".to_string()),
                Token::Option("b".to_string()),
                Token::Option("a".to_string()),
                Token::Option("r".to_string()),
            )
        );
    }

    #[test]
    fn plain_dash_is_not_an_option() {
        assert_eq!(
            tokenize_shell(r#"- foo -"#).unwrap(),
            vec!(
                Token::Text("-".to_string()),
                Token::Text("foo".to_string()),
                Token::Text("-".to_string()),
            )
        );
    }

    #[test]
    fn plain_dash_dash_is_not_an_option() {
        assert_eq!(
            tokenize_shell(r#"-- foo --"#).unwrap(),
            vec!(
                Token::Text("--".to_string()),
                Token::Text("foo".to_string()),
                Token::Text("--".to_string()),
            )
        );
    }

    #[test]
    fn dash_inside_other_items() {
        assert_eq!(
            tokenize_shell(r#"--long-flag normal-text also--this"#).unwrap(),
            vec!(
                Token::Option("long-flag".to_string()),
                Token::Text("normal-text".to_string()),
                Token::Text("also--this".to_string()),
            )
        );
    }

    #[test]
    fn just_a_dash() {
        assert_eq!(
            tokenize_shell(r#"-"#).unwrap(),
            vec!(
                Token::Text("-".to_string()),
            )
        );
    }

    #[test]
    fn just_dash_dash() {
        assert_eq!(
            tokenize_shell(r#"--"#).unwrap(),
            vec!(
                Token::Text("--".to_string()),
            )
        );
    }


    #[test]
    fn empty() {
        assert_eq!(
            tokenize_shell(r#""#).unwrap(),
            vec!()
        );
    }

    #[test]
    fn trailing_space() {
        assert_eq!(
            tokenize_shell(r#"foo "#).unwrap(),
            vec!(
                Token::Text("foo".to_string()),
            )
        );
    }

    #[test]
    fn leading_space() {
        let xxx = tokenize_shell(r#" foo"#).unwrap();
        println!("{:?}", xxx);
        assert_eq!(
            tokenize_shell(r#" foo"#).unwrap(),
            vec!(
                Token::Text("foo".to_string()),
            )
        );
    }

    #[test]
    fn double_space() {
        let xxx = tokenize_shell(r#" foo"#).unwrap();
        println!("{:?}", xxx);
        assert_eq!(
            tokenize_shell(r#"foo  bar"#).unwrap(),
            vec!(
                Token::Text("foo".to_string()),
                Token::Text("bar".to_string()),
            )
        );
    }
}
