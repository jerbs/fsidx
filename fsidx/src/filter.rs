#[derive(Clone, Debug)]
pub enum FilterToken {
    Text(String),
    Next(String),
    CaseSensitive,
    CaseInSensitive,    // default
    AnyOrder,           // default
    SameOrder,
    WholePath,          // default
    LastElement,
    SmartSpaces(bool),  // default: on  
}

pub fn compile(filter: &[FilterToken]) -> Vec<FilterToken> {
    let filter = prepare_case_insensitive(filter);
    let filter = prepare_backtracking(filter);
    filter
}

fn prepare_case_insensitive(filter: &[FilterToken]) -> Vec<FilterToken> {
    let mut b_case_sensitive = false;
    let filter: Vec<FilterToken> = filter.iter().map( |token|
        match token {
            FilterToken::CaseSensitive   => {b_case_sensitive = true; token.clone()},
            FilterToken::CaseInSensitive => {b_case_sensitive = false; token.clone()},
            FilterToken::Text(text) if b_case_sensitive == false => FilterToken::Text(text.to_lowercase()),
            FilterToken::Next(_text) => panic!(),
            _ => token.clone(),
        }
    ).collect();
    filter
}

fn prepare_backtracking(filter: Vec<FilterToken>) -> Vec<FilterToken> {
    let mut result = Vec::new();
    let mut b_smart_spaces = true;
    let mut b_same_order = false;
    for token in filter {
        match token {
            FilterToken::SmartSpaces(on) => {b_smart_spaces = on;},
            FilterToken::SameOrder => {b_same_order = true; result.push(token);}
            FilterToken::AnyOrder => {b_same_order = false; result.push(token);}
            FilterToken::Text(text) if b_smart_spaces => {expand_smart_spaces(text, b_same_order, &mut result)},
            _ => result.push(token),
        }
    }
    result
}

fn expand_smart_spaces(text: String, b_same_order: bool, filter: &mut Vec<FilterToken>) {
    let mut first = true;
    let b_stored_same_order = b_same_order;
    for part in text.split(&[' ', '-', '_']) {
        if !first {
            filter.push(FilterToken::SameOrder);
        }
        if first {
            filter.push(FilterToken::Text(part.to_string()));
            first = false;
        } else {
            filter.push(FilterToken::Next(part.to_string()));
        }
    }
    if !b_stored_same_order {
        filter.push(FilterToken::AnyOrder);
    }
}

struct State {
    index: usize,
    pos: usize
}

pub fn apply(text: &str, filter: &[FilterToken]) -> bool {
    let lower_text: String = text.to_lowercase();
    let (last_text, lower_last_text) = if let Some(pos_last_slash) = text.rfind('/') {
        let last_text = &text[pos_last_slash+1..];
        let lower_last_text = &lower_text[pos_last_slash+1..];
        (last_text, lower_last_text)
    } else {
        (text, &lower_text[..])
    };
    
    let mut pos: usize = 0;
    let mut index = 0;
    let mut b_case_sensitive = false;
    let mut b_same_order = false;
    let mut b_last_element = false;
    let filter_len = filter.len();
    
    let mut back_tracking = State { index: 0, pos: 0 };
    while index < filter_len {
        let token = &filter[index];
        if let FilterToken::Text(_) = token {
            back_tracking = State { index, pos };
        }
        index = index + 1;
        if ! match token {
            FilterToken::Text(pattern) if  b_case_sensitive &&  b_same_order &&  b_last_element => if let Some(npos) = last_text[pos..].find(pattern)       {pos = pos + npos + pattern.len(); true} else {false},
            FilterToken::Text(pattern) if !b_case_sensitive &&  b_same_order &&  b_last_element => if let Some(npos) = lower_last_text[pos..].find(pattern) {pos = pos + npos + pattern.len(); true} else {false},
            FilterToken::Text(pattern) if  b_case_sensitive && !b_same_order &&  b_last_element => if let Some(npos) = last_text.find(pattern)              {pos = pos + npos + pattern.len(); true} else {false},
            FilterToken::Text(pattern) if !b_case_sensitive && !b_same_order &&  b_last_element => if let Some(npos) = lower_last_text.find(pattern)        {pos = pos + npos + pattern.len(); true} else {false},
            FilterToken::Text(pattern) if  b_case_sensitive &&  b_same_order && !b_last_element => if let Some(npos) = text[pos..].find(pattern)            {pos = pos + npos + pattern.len(); true} else {false},
            FilterToken::Text(pattern) if !b_case_sensitive &&  b_same_order && !b_last_element => if let Some(npos) = lower_text[pos..].find(pattern)      {pos = pos + npos + pattern.len(); true} else {false},
            FilterToken::Text(pattern) if  b_case_sensitive && !b_same_order && !b_last_element => if let Some(npos) = text.find(pattern)                   {pos = pos + npos + pattern.len(); true} else {false},
            FilterToken::Text(pattern) if !b_case_sensitive && !b_same_order && !b_last_element => if let Some(npos) = lower_text.find(pattern)             {pos = pos + npos + pattern.len(); true} else {false},
            FilterToken::Next(pattern) if  b_case_sensitive => {let s = apply_next(State {index, pos}, pattern, &text, &back_tracking); index = s.index; pos = s.pos; true},  // TODO: use destructuring_assignment
            FilterToken::Next(pattern) if !b_case_sensitive => {let s = apply_next(State {index, pos}, pattern, &lower_text, &back_tracking); index = s.index; pos = s.pos; true},
            FilterToken::Text(_) => false,
            FilterToken::Next(_) => false,
            FilterToken::CaseSensitive => {b_case_sensitive = true; true},
            FilterToken::CaseInSensitive => {b_case_sensitive = false; true},
            FilterToken::AnyOrder => {b_same_order = false; true},
            FilterToken::SameOrder => {b_same_order = true; true},
            FilterToken::WholePath => {b_last_element = false; true},
            FilterToken::LastElement => {b_last_element = true; true},
            FilterToken::SmartSpaces(_) => true,
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

    static DATA: [&str; 7] = [S0, S1, S2, S3, S4, S5, S6];

    fn process(flt: &[FilterToken]) -> Vec<String> {
        let flt = compile(flt);
        DATA.iter().filter(|entry: &&&str| apply(entry, &flt)).map(|x: &&str| String::from(*x)).collect()
    }

    static EMPTY: [&str; 0] = [];
    fn t(s: &str) -> FilterToken { FilterToken::Text(String::from(s)) }

    #[test]
    fn all_with_empty_string() {
        assert_eq!(process(&[t("")]), [S0, S1, S2, S3, S4, S5, S6]);
    }

    #[test]
    fn all_with_empty_list() {
        assert_eq!(process(&[]), [S0, S1, S2, S3, S4, S5, S6]);
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
        assert_eq!(apply("foo bar", &compile(&[FilterToken::SameOrder, FilterToken::Text("foo".to_string())])), true);
        assert_eq!(apply("foo bar", &compile(&[FilterToken::SameOrder, FilterToken::Text("foo".to_string()), FilterToken::Text("foo".to_string())])), false);
        assert_eq!(apply("foo bar baz", &compile(&[FilterToken::SameOrder, FilterToken::Text("foo".to_string()), FilterToken::Text("baz".to_string())])), true);
        assert_eq!(apply("foo bar baz", &compile(&[FilterToken::Text("foo baz".to_string())])), false);
        assert_eq!(apply("fOO bar baZ", &compile(&[FilterToken::Text("Foo Bar".to_string())])), true);
        assert_eq!(apply("foo foo", &compile(&[FilterToken::Text("foo foo".to_string())])), true);
        assert_eq!(apply("foo bar", &compile(&[FilterToken::Text("foo foo".to_string())])), false);
        assert_eq!(apply("foo-foo", &compile(&[FilterToken::Text("foo foo".to_string())])), true);
        assert_eq!(apply("foo_foo", &compile(&[FilterToken::Text("foo foo".to_string())])), true);
    }

    #[test]
    fn smart_space() {
        assert_eq!(apply("foo bar abc baz", &compile(&[FilterToken::Text("foo baz".to_string())])), false);
        assert_eq!(apply("foo bar abc baz", &compile(&[FilterToken::Text("bar abc".to_string())])), true);
        assert_eq!(apply("foo-bar-abc-baz", &compile(&[FilterToken::Text("bar abc".to_string())])), true);
        assert_eq!(apply("foo_bar_abc_baz", &compile(&[FilterToken::Text("bar abc".to_string())])), true);
        assert_eq!(apply("foo_bar_abc_baz", &compile(&[FilterToken::Text("bar-abc".to_string())])), true);
        assert_eq!(apply("foo bar abc baz", &compile(&[FilterToken::Text("bar_abc".to_string())])), true);
    }

    #[test]
    fn retry_on_failure_with_next() {
        assert_eq!(apply("foo bar baz", &compile(&[FilterToken::Text("b-a-r".to_string())])), true);
        assert_eq!(apply("foo baz bar", &compile(&[FilterToken::Text("b-a-r".to_string())])), true);
        assert_eq!(apply("foo baz bax", &compile(&[FilterToken::Text("b-a-r".to_string())])), false);
    }

    #[test]
    fn back_tracking_skip_multibyte_characters() {
        let text = "äaäa";   //  [61, CC, 88, 61, C3, A4, 61]
        // 0x61      : a
        // 0xCC, 0x88: Trema for previous letter (https://www.compart.com/de/unicode/U+0308)
        // 0xC3, 0xA4: ä
        // println!("{:02X?}", text.bytes());
        assert_eq!(apply(text, &compile(&[FilterToken::Text("a-b".to_string())])), false);
    }
}
