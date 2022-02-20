pub fn tokenize<'a>(text: &'a str) -> Tokenizer<'a> {
    Tokenizer {
        text
    }
}

pub struct Tokenizer<'a> {
    text: &'a str,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Text(String),
    Backslash(String),
    Option(String),
}

impl<'a> IntoIterator for Tokenizer<'a> {
    type Item = Token;

    type IntoIter = TokenIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            remainder: self.text,
        }
    }
}

pub struct TokenIterator<'a> {
    remainder: &'a str,
}

impl<'a> Iterator for TokenIterator<'a> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        let mut item = String::new();
        let mut in_token = false;
        let mut quoted = false;
        let mut backslashed = false;
        let mut backslash_command = false;
        let mut option_string = false;
        for (pos, ch) in self.remainder.char_indices() {
            let mut done = false;
            // println!("- {}, {}", pos, ch);
            match (ch,                     in_token, quoted, backslashed) {
                (' ' | '\t' | '\n' | '\r',    false,  false,       false) => {},                                               // Leading whitespaces
                (' ' | '\t' | '\n' | '\r',     true,  false,       false) => {done = true;},                                   // Trailing whitespaces
                ('\\'                    ,     true,      _,       false) => {backslashed = true;},                            // A backslash within quotes
                ('\\'                    ,    false,      _,       false) => {backslashed = true; backslash_command = true;},  // Start of a backslash command
                ('-'                     ,    false,      _,       false) => {option_string = true; in_token = true;},         // Start of an option
                ('"'                     ,     true,   true,       false) => {quoted = false;},                                // End of quoted string
                ('"'                     ,        _,      _,       false) => {quoted = true; in_token = true;},                // Start of quoted string
                ('"'                     ,        _,      _,        true) => {item.push('"'); backslashed = false;},                       // Backslash escaped quote
                (ch                 ,        _,      _,        true) => {item.push(map(ch)); in_token = true; backslashed = false;},  // A backslashed character
                (ch                 ,        _,      _,       false) => {item.push(ch); in_token = true;},                               // Any character
            }
            if done {
                self.remainder = &self.remainder[pos..];
                if item == "" {option_string = false; item = "-".to_string();}
                return match (backslash_command, option_string) {
                    (true, _) => Some(Token::Backslash(item)),
                    (_, true) => Some(Token::Option(item)),
                    (_, _) => Some(Token::Text(item)),
                };
            }
        }
        self.remainder = "";
        if in_token || backslashed || option_string {
            if item == "" {option_string = false; item = "-".to_string();}
            return match (backslash_command, option_string) {
                (true, _) => Some(Token::Backslash(item)),
                (_, true) => Some(Token::Option(item)),
                (_, _) => Some(Token::Text(item)),
            };
        } else {
            None
        }
    }
}

fn map(ch: char) -> char {
    match ch {
        'n' => '\n',
        'r' => '\r',
        't' => '\t',
        ch => ch,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    #[test]
    fn splitting_on_whitespace() {
        let text = indoc! { r#"This text is split on white spaces. 안녕하세요 end"# };
        let mut tokenizer = tokenize(text).into_iter();
        assert_eq!(Some(Token::Text("This".to_string())), tokenizer.next());
        assert_eq!(Some(Token::Text("text".to_string())), tokenizer.next());
        assert_eq!(Some(Token::Text("is".to_string())), tokenizer.next());
        assert_eq!(Some(Token::Text("split".to_string())), tokenizer.next());
        assert_eq!(Some(Token::Text("on".to_string())), tokenizer.next());
        assert_eq!(Some(Token::Text("white".to_string())), tokenizer.next());
        assert_eq!(Some(Token::Text("spaces.".to_string())), tokenizer.next());
        assert_eq!(Some(Token::Text("안녕하세요".to_string())), tokenizer.next());
        assert_eq!(Some(Token::Text("end".to_string())), tokenizer.next());
        assert_eq!(None, tokenizer.next());
    }

    #[test]
    fn for_loop() {
        let mut tokens: Vec<Token> = Vec::new();
        let text = indoc! { r#"This text is split on white spaces."# };
        let tokenizer = tokenize(text);
        for token in tokenizer {
            // println!("- {}", token);
            tokens.push(token);
        }
        assert_eq!(tokens, vec![
            Token::Text("This".to_string()),
            Token::Text("text".to_string()),
            Token::Text("is".to_string()),
            Token::Text("split".to_string()),
            Token::Text("on".to_string()),
            Token::Text("white".to_string()),
            Token::Text("spaces.".to_string())
            ])
    }

    #[test]
    fn quoted_string() {
        let text = indoc! { r#"Herr "Max Mustermann""# };
        let tokens: Vec<_> = tokenize(text).into_iter().collect();
        assert_eq!(tokens, vec![
            Token::Text("Herr".to_string()),
            Token::Text("Max Mustermann".to_string())
            ]);
    }

    #[test]
    fn joined_quoted_string() {
        let text = indoc! { r#"Herr "Max ""Mustermann""# };
        let tokens: Vec<_> = tokenize(text).into_iter().collect();
        assert_eq!(tokens, vec![
            Token::Text("Herr".to_string()),
            Token::Text("Max Mustermann".to_string())
            ]);
    }

    #[test]
    fn multiple_quoted_string() {
        let text = indoc! { r#"Herr "Max ""Mustermann" "Peter Müller""# };
        let tokens: Vec<_> = tokenize(text).into_iter().collect();
        assert_eq!(tokens, vec![
            Token::Text("Herr".to_string()),
            Token::Text("Max Mustermann".to_string()),
            Token::Text("Peter Müller".to_string())
            ]);
    }

    #[test]
    fn escaped_quotes() {
        let text = indoc! { r#"This is a \"token\" with quotes."# };
        let tokens: Vec<_> = tokenize(text).into_iter().collect();
        assert_eq!(tokens, vec![
            Token::Text("This".to_string()),
            Token::Text("is".to_string()),
            Token::Text("a".to_string()),
            Token::Backslash("\"token\"".to_string()),
            Token::Text("with".to_string()),
            Token::Text("quotes.".to_string())
            ]);
    }

    #[test]
    fn quoted_escaped_quotes() {
        let text = indoc! { r#"This is a "\"token\"" with quotes."# };
        let tokens: Vec<_> = tokenize(text).into_iter().collect();
        assert_eq!(tokens, vec![
            Token::Text("This".to_string()),
            Token::Text("is".to_string()),
            Token::Text("a".to_string()),
            Token::Text("\"token\"".to_string()),
            Token::Text("with".to_string()),
            Token::Text("quotes.".to_string())
            ]);
    }

    #[test]
    fn backslahed_characters() {
        let text = indoc! { r#"\a\b\c"123" 12\ 34 a"bc"de\"fg \hi " \hi"# };
        let tokens: Vec<_> = tokenize(text).into_iter().collect();
        assert_eq!(tokens, vec![
            Token::Backslash("abc123".to_string()),
            Token::Text("12 34".to_string()),
            Token::Text("abcde\"fg".to_string()),
            Token::Backslash("hi".to_string()),
            Token::Text(" hi".to_string()),
            ]);
    }

    #[test]
    fn option_strings() {
        let text = indoc! { r#"-a-b-c"123" 12- 34 a"bc"de-"fg -hi " -hi"# };
        let tokens: Vec<_> = tokenize(text).into_iter().collect();
        assert_eq!(tokens, vec![
            Token::Option("a-b-c123".to_string()),
            Token::Text("12-".to_string()),
            Token::Text("34".to_string()),
            Token::Text("abcde-fg -hi ".to_string()),
            Token::Option("hi".to_string()),
            ]);
    }

    #[test]
    fn single_dash_is_not_an_option() {
        let text = indoc! { r#"abc - def"# };
        let tokens: Vec<_> = tokenize(text).into_iter().collect();
        assert_eq!(tokens, vec![
            Token::Text("abc".to_string()),
            Token::Text("-".to_string()),
            Token::Text("def".to_string()),
            ]);
    }
    #[test]
    fn just_a_single_dash() {
        let text = indoc! { r#"-"# };
        let tokens: Vec<_> = tokenize(text).into_iter().collect();
        assert_eq!(tokens, vec![
            Token::Text("-".to_string()),
            ]);
    }
}
