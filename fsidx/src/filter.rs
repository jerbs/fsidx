use glob::{Pattern, MatchOptions, PatternError};

#[derive(Clone, Debug, PartialEq)]
pub enum FilterToken {
    Text(String),
    CaseSensitive,
    CaseInSensitive,    // default
    AnyOrder,           // default
    SameOrder,
    WholePath,          // default
    LastElement,
    SmartSpaces(bool),  // default: on
    RequireLiteralSeparator(bool),  // default: off
    RequireLiteralLeadingDot(bool), // default: off
    Smart,
    Glob,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CompiledFilterToken {
    Glob(Pattern, MatchOptions),
    SmartText(String),
    SmartNext(String),
    CaseSensitive,
    CaseInSensitive,    // default
    AnyOrder,           // default
    SameOrder,
    WholePath,          // default
    LastElement,
}

#[derive(Clone, Debug, PartialEq)]
enum Mode {
    Smart,
    Glob,
}

pub fn compile(filter: &[FilterToken]) -> Result<Vec<CompiledFilterToken>, PatternError> {
    let mut result = Vec::new();
    let mut mode: Mode = Mode::Smart;
    let mut case_sensitive = false;
    let mut smart_spaces = true;
    let mut same_order = false;
    let mut match_options = MatchOptions {
        case_sensitive,
        require_literal_separator: false,
        require_literal_leading_dot: false,
    };
    for token in filter {
        match token {
            FilterToken::CaseSensitive   => { case_sensitive = true; match_options.case_sensitive = true; result.push(CompiledFilterToken::CaseSensitive); },
            FilterToken::CaseInSensitive => { case_sensitive = false; match_options.case_sensitive = false; result.push(CompiledFilterToken::CaseInSensitive); },
            FilterToken::Text(text) if mode == Mode::Smart &&  case_sensitive &&  smart_spaces => { expand_smart_spaces(text.clone(), same_order, &mut result); },
            FilterToken::Text(text) if mode == Mode::Smart &&  case_sensitive && !smart_spaces => { result.push(CompiledFilterToken::SmartText(text.clone())); },
            FilterToken::Text(text) if mode == Mode::Smart && !case_sensitive &&  smart_spaces => { expand_smart_spaces(text.to_lowercase(), same_order, &mut result); },
            FilterToken::Text(text) if mode == Mode::Smart && !case_sensitive && !smart_spaces => { result.push(CompiledFilterToken::SmartText(text.to_lowercase())); },
            FilterToken::Text(text) if mode == Mode::Glob => { result.push(CompiledFilterToken::Glob(Pattern::new(text.as_str())?, match_options.clone())) },
            FilterToken::Text(_) => { todo!(); },   // Getting a warning without this line.
            FilterToken::AnyOrder => { same_order = false; result.push(CompiledFilterToken::AnyOrder); }
            FilterToken::SameOrder => { same_order = true; result.push(CompiledFilterToken::SameOrder); }
            FilterToken::WholePath => { result.push(CompiledFilterToken::WholePath); },
            FilterToken::LastElement => { result.push(CompiledFilterToken::LastElement); },
            FilterToken::SmartSpaces(on) => { smart_spaces = *on; },
            FilterToken::RequireLiteralSeparator(on) => { match_options.require_literal_separator = *on; },
            FilterToken::RequireLiteralLeadingDot(on) => {match_options.require_literal_leading_dot = *on; },
            FilterToken::Smart => { mode = Mode::Smart; },
            FilterToken::Glob => { mode = Mode::Glob; },
        }
    }
    Ok(result)
}

fn expand_smart_spaces(text: String, mut b_same_order: bool, filter: &mut Vec<CompiledFilterToken>) {
    let mut first = true;
    let b_stored_same_order = b_same_order;
    for part in text.split(&[' ', '-', '_']) {
        if !part.is_empty() {
            if !first && !b_same_order {
                b_same_order = true;
                filter.push(CompiledFilterToken::SameOrder);
            }
            if first {
                filter.push(CompiledFilterToken::SmartText(part.to_string()));
                first = false;
            } else {
                filter.push(CompiledFilterToken::SmartNext(part.to_string()));
            }    
        }
    }
    if !b_stored_same_order && b_same_order {
        filter.push(CompiledFilterToken::AnyOrder);
    }
}

struct State {
    index: usize,
    pos: usize
}

pub fn apply(text: &str, filter: &[CompiledFilterToken]) -> bool {
    let lower_text: String = text.to_lowercase();
    let (last_text, lower_last_text) = if let Some(pos_last_slash) = text.rfind('/') {
        let last_text = &text[pos_last_slash+1..];
        let lower_last_text = &lower_text[pos_last_slash+1..];
        (last_text, lower_last_text)
    } else {
        (text, &lower_text[..])
    };
    
    let mut pos: usize = 0;     // FIXME: Do I need to split this into an whole path position and a last element position? Add a test that switches between the two modes.
    let mut index = 0;
    let mut b_case_sensitive = false;
    let mut b_same_order = false;
    let mut b_last_element = false;
    let filter_len = filter.len();
    
    let mut back_tracking = State { index: 0, pos: 0 };
    while index < filter_len {
        let token = &filter[index];
        if let CompiledFilterToken::SmartText(_) = token {
            back_tracking = State { index, pos };
        }
        index = index + 1;
        if ! match token {
            CompiledFilterToken::SmartText(pattern) if  b_case_sensitive &&  b_same_order &&  b_last_element => if let Some(npos) = last_text[pos..].find(pattern)       {pos = pos + npos + pattern.len(); true} else {false},
            CompiledFilterToken::SmartText(pattern) if !b_case_sensitive &&  b_same_order &&  b_last_element => if let Some(npos) = lower_last_text[pos..].find(pattern) {pos = pos + npos + pattern.len(); true} else {false},
            CompiledFilterToken::SmartText(pattern) if  b_case_sensitive && !b_same_order &&  b_last_element => if let Some(npos) = last_text.find(pattern)              {pos =       npos + pattern.len(); true} else {false},
            CompiledFilterToken::SmartText(pattern) if !b_case_sensitive && !b_same_order &&  b_last_element => if let Some(npos) = lower_last_text.find(pattern)        {pos =       npos + pattern.len(); true} else {false},
            CompiledFilterToken::SmartText(pattern) if  b_case_sensitive &&  b_same_order && !b_last_element => if let Some(npos) = text[pos..].find(pattern)            {pos = pos + npos + pattern.len(); true} else {false},
            CompiledFilterToken::SmartText(pattern) if !b_case_sensitive &&  b_same_order && !b_last_element => if let Some(npos) = lower_text[pos..].find(pattern)      {pos = pos + npos + pattern.len(); true} else {false},
            CompiledFilterToken::SmartText(pattern) if  b_case_sensitive && !b_same_order && !b_last_element => if let Some(npos) = text.find(pattern)                   {pos =       npos + pattern.len(); true} else {false},
            CompiledFilterToken::SmartText(pattern) if !b_case_sensitive && !b_same_order && !b_last_element => if let Some(npos) = lower_text.find(pattern)             {pos =       npos + pattern.len(); true} else {false},
            CompiledFilterToken::SmartNext(pattern) if  b_case_sensitive => {let s = apply_next(State {index, pos}, pattern, &text, &back_tracking); index = s.index; pos = s.pos; true},  // TODO: use destructuring_assignment
            CompiledFilterToken::SmartNext(pattern) if !b_case_sensitive => {let s = apply_next(State {index, pos}, pattern, &lower_text, &back_tracking); index = s.index; pos = s.pos; true},
            CompiledFilterToken::SmartText(_) => false,
            CompiledFilterToken::SmartNext(_) => false,
            CompiledFilterToken::CaseSensitive => {b_case_sensitive = true; true},
            CompiledFilterToken::CaseInSensitive => {b_case_sensitive = false; true},
            CompiledFilterToken::AnyOrder => {b_same_order = false; true},
            CompiledFilterToken::SameOrder => {b_same_order = true; true},
            CompiledFilterToken::WholePath => {b_last_element = false; true},
            CompiledFilterToken::LastElement => {b_last_element = true; true},
            CompiledFilterToken::Glob(pattern, options) => pattern.matches_with(text, *options),
        } {
            return false
        }
    }
    true
}

fn apply_next(State {mut index, mut pos }: State, pattern: &String, text: &str, back_tracking: &State) -> State {
    let skip = skip_separator(&text[pos..]);
    if text[pos+skip..].starts_with(pattern) {
        pos = pos + skip + pattern.len();
    } else {
        index = back_tracking.index;
        pos = back_tracking.pos;
        if let Some(ch) = text[pos..].chars().next() {
            pos += ch.len_utf8();
        }
    };
    State { index, pos }
}

fn skip_separator(text: &str) -> usize {
    let mut chars = text.chars();
    if let Some(first) = chars.next() {
        match first {
            ' ' => first.len_utf8(),
            '-' => first.len_utf8(),
            '_' => first.len_utf8(),
            _ => 0,
        }    
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static S0: &str = "/ABCDEF";
    static S1: &str = "/ABC/DEFGHIJKLMN/OPQRSTUVWXYZ/eins";
    static S2: &str = "/abc/defghijklmn/opqrstuvwxyz/zwei";
    static S3: &str = "/AbCdEfGh/IjKlMn/OpQrStUvWxYz/drei";
    static S4: &str = "OpQrStUvWxYz/IjKlMn/AbCdEfGh/vier";
    static S5: &str = "/klmn";
    static S6: &str = "/xyz";
    static S7: &str = "/path/to/hidden/.file.txt";

    static DATA: [&str; 8] = [S0, S1, S2, S3, S4, S5, S6, S7];

    fn process(flt: &[FilterToken]) -> Vec<String> {
        let flt = compile(flt).unwrap();
        DATA.iter().filter(|entry: &&&str| apply(entry, &flt)).map(|x: &&str| String::from(*x)).collect()
    }

    static EMPTY: [&str; 0] = [];
    fn t(s: &str) -> FilterToken { FilterToken::Text(String::from(s)) }

    #[test]
    fn all_with_empty_string() {
        assert_eq!(process(&[t("")]), [S0, S1, S2, S3, S4, S5, S6, S7]);
    }

    #[test]
    fn all_with_empty_list() {
        assert_eq!(process(&[]), [S0, S1, S2, S3, S4, S5, S6, S7]);
    }

    #[test]
    fn default() {
       assert_eq!(process(&[t("Y"), t("G"), t("A")]), [S1, S2, S3, S4]);
    }
     
    #[test]
    fn case_insensitive_any_order_whole_path() {
        assert_eq!(process(&[FilterToken::CaseInSensitive, FilterToken::AnyOrder, FilterToken::WholePath, t("Y"), t("A"), t("G")]), [S1, S2, S3, S4]);
        assert_eq!(process(&[FilterToken::CaseInSensitive, FilterToken::AnyOrder, FilterToken::WholePath, t("a"), t("a"), t("g")]), [S1, S2, S3, S4]);
    }

    #[test]
    fn case_sensitive_any_order_whole_path() {
        assert_eq!(process(&[FilterToken::CaseSensitive, FilterToken::AnyOrder, FilterToken::WholePath, t("Y"), t("A"), t("G")]), [S1, S3, S4]);
        assert_eq!(process(&[FilterToken::CaseSensitive, FilterToken::AnyOrder, FilterToken::WholePath, t("y"), t("A"), t("G")]), EMPTY);
    }

    #[test]
    fn case_insensitive_same_order_whole_path() {
        assert_eq!(process(&[FilterToken::CaseInSensitive, FilterToken::SameOrder, FilterToken::WholePath, t("Y"), t("A"), t("G")]), [S4]);
        assert_eq!(process(&[FilterToken::CaseInSensitive, FilterToken::SameOrder, FilterToken::WholePath, t("y"), t("a"), t("g")]), [S4]);
    }

    #[test]
    fn case_sensitive_same_order_whole_path() {
        assert_eq!(process(&[FilterToken::CaseSensitive, FilterToken::SameOrder, FilterToken::WholePath, t("Y"), t("A"), t("G")]), [S4]);
        assert_eq!(process(&[FilterToken::CaseSensitive, FilterToken::SameOrder, FilterToken::WholePath, t("Y"), t("a"), t("G")]), EMPTY);
    }

    #[test]
    fn case_insensitive_any_order_last_component() {
        assert_eq!(process(&[FilterToken::CaseInSensitive, FilterToken::AnyOrder, FilterToken::LastElement, t("e"), t("d")]), [S0, S3]);
        assert_eq!(process(&[FilterToken::CaseInSensitive, FilterToken::AnyOrder, FilterToken::LastElement, t("E"), t("d")]), [S0, S3]);
    }

    #[test]
    fn case_sensitive_any_order_last_component() {
        assert_eq!(process(&[FilterToken::CaseSensitive, FilterToken::AnyOrder, FilterToken::LastElement, t("e"), t("d")]), [S3]);
        assert_eq!(process(&[FilterToken::CaseSensitive, FilterToken::AnyOrder, FilterToken::LastElement, t("E"), t("D")]), [S0]);
    }

    #[test]
    fn case_insensitive_same_order_last_component() {
        assert_eq!(process(&[FilterToken::CaseInSensitive, FilterToken::SameOrder, FilterToken::LastElement, t("e"), t("d")]), EMPTY);
        assert_eq!(process(&[FilterToken::CaseInSensitive, FilterToken::SameOrder, FilterToken::LastElement, t("d"), t("e")]), [S0, S3]);
        assert_eq!(process(&[FilterToken::CaseInSensitive, FilterToken::SameOrder, FilterToken::LastElement, t("d"), t("E")]), [S0, S3]);
    }

    #[test]
    fn case_sensitive_same_order_last_component() {
        assert_eq!(process(&[FilterToken::CaseSensitive, FilterToken::SameOrder, FilterToken::LastElement, t("e"), t("d")]), EMPTY);
        assert_eq!(process(&[FilterToken::CaseSensitive, FilterToken::SameOrder, FilterToken::LastElement, t("d"), t("e")]), [S3]);
        assert_eq!(process(&[FilterToken::CaseSensitive, FilterToken::SameOrder, FilterToken::LastElement, t("e"), t("d")]), EMPTY);
        assert_eq!(process(&[FilterToken::CaseSensitive, FilterToken::SameOrder, FilterToken::LastElement, t("D"), t("E")]), [S0]);
        assert_eq!(process(&[FilterToken::CaseSensitive, FilterToken::SameOrder, FilterToken::LastElement, t("E"), t("D")]), EMPTY);
    }

    #[test]
    fn continue_after_last_match() {
        assert_eq!(apply("foo bar", &compile(&[FilterToken::SameOrder, t("foo")]).unwrap()), true);
        assert_eq!(apply("foo bar", &compile(&[FilterToken::SameOrder, t("foo"), t("foo")]).unwrap()), false);
        assert_eq!(apply("foo bar baz", &compile(&[FilterToken::SameOrder, t("foo"), t("baz")]).unwrap()), true);
        assert_eq!(apply("foo bar baz", &compile(&[t("foo baz")]).unwrap()), false);
        assert_eq!(apply("fOO bar baZ", &compile(&[t("Foo Bar")]).unwrap()), true);
        assert_eq!(apply("foo foo", &compile(&[t("foo foo")]).unwrap()), true);
        assert_eq!(apply("foo bar", &compile(&[t("foo foo")]).unwrap()), false);
        assert_eq!(apply("foo-foo", &compile(&[t("foo foo")]).unwrap()), true);
        assert_eq!(apply("foo_foo", &compile(&[t("foo foo")]).unwrap()), true);
    }

    #[test]
    fn smart_space() {
        assert_eq!(apply("foo bar abc baz", &compile(&[t("foo baz")]).unwrap()), false);
        assert_eq!(apply("foo bar abc baz", &compile(&[t("bar abc")]).unwrap()), true);
        assert_eq!(apply("foo-bar-abc-baz", &compile(&[t("bar abc")]).unwrap()), true);
        assert_eq!(apply("foo_bar_abc_baz", &compile(&[t("bar abc")]).unwrap()), true);
        assert_eq!(apply("foo_bar_abc_baz", &compile(&[t("bar-abc")]).unwrap()), true);
        assert_eq!(apply("foo bar abc baz", &compile(&[t("bar_abc")]).unwrap()), true);
    }

    #[test]
    fn retry_on_failure_with_next() {
        assert_eq!(apply("foo bar baz", &compile(&[t("b-a-r")]).unwrap()), true);
        assert_eq!(apply("foo baz bar", &compile(&[t("b-a-r")]).unwrap()), true);
        assert_eq!(apply("foo baz bax", &compile(&[t("b-a-r")]).unwrap()), false);
    }

    #[test]
    fn back_tracking_skip_multibyte_characters() {
        let text = "äaäa";   //  [61, CC, 88, 61, C3, A4, 61]
        // 0x61      : a
        // 0xCC, 0x88: Trema for previous letter (https://www.compart.com/de/unicode/U+0308)
        // 0xC3, 0xA4: ä
        // println!("{:02X?}", text.bytes());
        assert_eq!(apply(text, &compile(&[t("a-b")]).unwrap()), false);
    }

    #[test]
    fn position_calculation_same_order() {
        let text = "              a            bc";
        for a in &[CompiledFilterToken::CaseInSensitive, CompiledFilterToken::CaseInSensitive] {
            for b in &[CompiledFilterToken::WholePath, CompiledFilterToken::LastElement] {
                assert_eq!(apply(text, &[
                    a.clone(),
                    b.clone(),
                    CompiledFilterToken::SameOrder,
                    CompiledFilterToken::SmartText("a".to_string()),
                    CompiledFilterToken::SmartText("b".to_string()),
                    CompiledFilterToken::SmartNext("c".to_string())
                ]), true);
            }
        }
    }

    #[test]
    fn position_calculation_any_order() {
        let text = "              bc            a";
        for a in &[CompiledFilterToken::CaseInSensitive, CompiledFilterToken::CaseInSensitive] {
            for b in &[CompiledFilterToken::WholePath, CompiledFilterToken::LastElement] {
                assert_eq!(apply(text, &[
                    a.clone(),
                    b.clone(),
                    CompiledFilterToken::AnyOrder,
                    CompiledFilterToken::SmartText("a".to_string()),
                    CompiledFilterToken::SmartText("b".to_string()),
                    CompiledFilterToken::SmartNext("c".to_string())
                ]), true);
            }
        }
    }

    #[test]
    fn compile_text_with_spaces() {
        let actual = compile( &[
            t("a b c d"),
            t("e"),
        ] ).unwrap();
        let expected = vec![
            CompiledFilterToken::SmartText("a".to_string()),
            CompiledFilterToken::SameOrder,
            CompiledFilterToken::SmartNext("b".to_string()),
            CompiledFilterToken::SmartNext("c".to_string()),
            CompiledFilterToken::SmartNext("d".to_string()),
            CompiledFilterToken::AnyOrder,
            CompiledFilterToken::SmartText("e".to_string()),
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn remove_empty_strings_after_expanding_smart_spaces() {
        let actual = compile(&[t("- a-b c- -d -")]).unwrap();
        let expected = vec![
            CompiledFilterToken::SmartText("a".to_string()),
            CompiledFilterToken::SameOrder,
            CompiledFilterToken::SmartNext("b".to_string()),
            CompiledFilterToken::SmartNext("c".to_string()),
            CompiledFilterToken::SmartNext("d".to_string()),
            CompiledFilterToken::AnyOrder,
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn glob_star() {
        assert_eq!(process(&[FilterToken::Glob, t("*i")]), [S2, S3]);
    }

    #[test]
    fn glob_recursive_wildcard() {
        assert_eq!(process(&[FilterToken::Glob,  FilterToken::RequireLiteralSeparator(false), t("/**/*s")]), [S1]);
    }

    #[test]
    fn glob_question_mark() {
        assert_eq!(process(&[FilterToken::Glob, t("*/???i")]), [S2, S3]);
    }

    #[test]
    fn glob_require_literal_separator() {
        assert_eq!(process(&[FilterToken::Glob, FilterToken::RequireLiteralSeparator(false), t("/*i")]), [S2, S3]);
        assert_eq!(process(&[FilterToken::Glob, FilterToken::RequireLiteralSeparator(true), t("/*i")]), EMPTY);
        assert_eq!(process(&[FilterToken::Glob, FilterToken::RequireLiteralSeparator(true), t("/*/*/*/*i")]), [S2, S3]);
        assert_eq!(process(&[FilterToken::Glob, FilterToken::RequireLiteralSeparator(true), t("/**/*i")]), [S2, S3]);
        assert_eq!(process(&[FilterToken::Glob, FilterToken::RequireLiteralSeparator(true), t("/**/eins")]), [S1]);
    }

    #[test]
    fn glob_require_literal_leading_dir() {
        assert_eq!(process(&[FilterToken::Glob, FilterToken::RequireLiteralLeadingDot(false), t("*.txt")]), [S7]);
        assert_eq!(process(&[FilterToken::Glob, FilterToken::RequireLiteralLeadingDot(true), t("*.txt")]), EMPTY);
        assert_eq!(process(&[FilterToken::Glob, FilterToken::RequireLiteralLeadingDot(true), t("*/*.txt")]), EMPTY);
        assert_eq!(process(&[FilterToken::Glob, FilterToken::RequireLiteralLeadingDot(true), t("*/.*.txt")]), [S7]);
    }
}
