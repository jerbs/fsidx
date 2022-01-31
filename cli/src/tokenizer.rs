pub fn tokenize<'a>(text: &'a str) -> Tokenizer<'a> {
    Tokenizer {
        text
    }
}

pub struct Tokenizer<'a> {
    text: &'a str,
} 

impl<'a> IntoIterator for Tokenizer<'a> {
    type Item = String;

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
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        let mut item = String::new();
        let mut in_token = false;
        let mut quoted = false;
        let mut backslashed = false;
        for (pos, ch) in self.remainder.char_indices() {
            let mut done = false;
            // println!("- {}, {}", pos, ch);
            match (ch,                     in_token, quoted, backslashed) {
                (' ' | '\t' | '\n' | '\r',    false,  false,       false) => {},                        // Leading whitespaces
                (' ' | '\t' | '\n' | '\r',     true,  false,       false) => {done = true;},            // Trailing whitespaces
                ('\\'                    ,        _,      _,       false) => {backslashed = true;},     // A backslash
                ('"'                     ,    false,      _,       false) => {quoted = true;},          // Start of quoted string
                ('"'                     ,     true,      _,       false) => {quoted = false;},         // End of quoted string
                ('"'                     ,        _,      _,        true) => {item.push('"'); backslashed = false;},                       // Backslash escaped quote
                (ch                 ,        _,      _,        true) => {item.push(map(ch)); in_token = true; backslashed = false;},  // A backslashed character
                (ch                 ,        _,      _,       false) => {item.push(ch); in_token = true;},                               // Any character
            }
            if done {
                self.remainder = &self.remainder[pos..];
                return Some(item);
            }
        }
        self.remainder = "";
        if !item.is_empty() {
            Some(item)
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
        assert_eq!(Some("This".to_string()), tokenizer.next());
        assert_eq!(Some("text".to_string()), tokenizer.next());
        assert_eq!(Some("is".to_string()), tokenizer.next());
        assert_eq!(Some("split".to_string()), tokenizer.next());
        assert_eq!(Some("on".to_string()), tokenizer.next());
        assert_eq!(Some("white".to_string()), tokenizer.next());
        assert_eq!(Some("spaces.".to_string()), tokenizer.next());
        assert_eq!(Some("안녕하세요".to_string()), tokenizer.next());
        assert_eq!(Some("end".to_string()), tokenizer.next());
        assert_eq!(None, tokenizer.next());
    }

    #[test]
    fn for_loop() {
        let mut tokens: Vec<String> = Vec::new();
        let text = indoc! { r#"This text is split on white spaces."# };
        let tokenizer = tokenize(text);
        for token in tokenizer {
            // println!("- {}", token);
            tokens.push(token);
        }
        assert_eq!(tokens, vec!["This", "text", "is", "split", "on", "white", "spaces."])
    }

    #[test]
    fn quoted_string() {
        let text = indoc! { r#"Herr "Max Mustermann""# };
        let tokens: Vec<_> = tokenize(text).into_iter().collect();
        assert_eq!(tokens, vec!["Herr", "Max Mustermann"]);
    }

    #[test]
    fn joined_quoted_string() {
        let text = indoc! { r#"Herr "Max ""Mustermann""# };
        let tokens: Vec<_> = tokenize(text).into_iter().collect();
        assert_eq!(tokens, vec!["Herr", "Max Mustermann"]);
    }

    #[test]
    fn multiple_quoted_string() {
        let text = indoc! { r#"Herr "Max ""Mustermann" "Peter Müller""# };
        let tokens: Vec<_> = tokenize(text).into_iter().collect();
        assert_eq!(tokens, vec!["Herr", "Max Mustermann", "Peter Müller"]);
    }

    #[test]
    fn escaped_quotes() {
        let text = indoc! { r#"This is a \"token\" with quotes."# };
        let tokens: Vec<_> = tokenize(text).into_iter().collect();
        assert_eq!(tokens, vec!["This", "is", "a", "\"token\"", "with", "quotes."]);
    }

    #[test]
    fn backslahed_characters() {
        let text = indoc! { r#"\a\b\c"123" 12\ 34 a"bc"de\"fg"# };
        let tokens: Vec<_> = tokenize(text).into_iter().collect();
        assert_eq!(tokens, vec!["abc123", "12 34", "abcde\"fg"]);
    }
}
