#[derive(Clone, Debug)]
pub enum FilterToken {
    Text(String),
    CaseSensitive,
    CaseInSensitive,    // default
    AnyOrder,           // default
    SameOrder,
    WholePath,          // default
    LastElement,
}

pub fn compile(filter: &[FilterToken]) -> Vec<FilterToken> {
    let mut result = Vec::new();
    let mut b_case_sensitive = false;

    for token in filter {
        let token: FilterToken = match token {
            FilterToken::CaseSensitive => {b_case_sensitive = true; token.clone()},
            FilterToken::CaseInSensitive => {b_case_sensitive = false; token.clone()},
            FilterToken::Text(text) if b_case_sensitive == false => {FilterToken::Text(text.to_lowercase())}
            _ => token.clone(),
        };
        result.push(token);
    }

    result
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
    let mut b_case_sensitive = false;
    let mut b_same_order = false;
    let mut b_last_element = false;
    
    for token in filter {
        if ! match token {
            FilterToken::Text(pattern) if  b_case_sensitive &&  b_same_order &&  b_last_element => if let Some(npos) = last_text[pos..].find(pattern)       {pos = npos; true} else {false},
            FilterToken::Text(pattern) if !b_case_sensitive &&  b_same_order &&  b_last_element => if let Some(npos) = lower_last_text[pos..].find(pattern) {pos = npos; true} else {false},
            FilterToken::Text(pattern) if  b_case_sensitive && !b_same_order &&  b_last_element => if let Some(npos) = last_text.find(pattern)              {pos = npos; true} else {false},
            FilterToken::Text(pattern) if !b_case_sensitive && !b_same_order &&  b_last_element => if let Some(npos) = lower_last_text.find(pattern)        {pos = npos; true} else {false},
            FilterToken::Text(pattern) if  b_case_sensitive &&  b_same_order && !b_last_element => if let Some(npos) = text[pos..].find(pattern)            {pos = npos; true} else {false},
            FilterToken::Text(pattern) if !b_case_sensitive &&  b_same_order && !b_last_element => if let Some(npos) = lower_text[pos..].find(pattern)      {pos = npos; true} else {false},
            FilterToken::Text(pattern) if  b_case_sensitive && !b_same_order && !b_last_element => if let Some(npos) = text.find(pattern)                   {pos = npos; true} else {false},
            FilterToken::Text(pattern) if !b_case_sensitive && !b_same_order && !b_last_element => if let Some(npos) = lower_text.find(pattern)             {pos = npos; true} else {false},
            FilterToken::Text(_) => false,
            FilterToken::CaseSensitive => {b_case_sensitive = true; true},
            FilterToken::CaseInSensitive => {b_case_sensitive = false; true},
            FilterToken::AnyOrder => {b_same_order = false; true},
            FilterToken::SameOrder => {b_same_order = true; true},
            FilterToken::WholePath => {b_last_element = false; true},
            FilterToken::LastElement => {b_last_element = true; true},
        } {
            return false
        }
    }

    true
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
}
