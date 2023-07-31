use globset::{GlobBuilder, GlobMatcher};
use crate::config::{LocateConfig, Mode};
use crate::locate::LocateError;

#[derive(Clone, Debug, PartialEq)]
pub enum FilterToken {
    Text(String),
    CaseSensitive,
    CaseInSensitive,         // default
    AnyOrder,                // default
    SameOrder,
    WholePath,               // default
    LastElement,
    SmartSpaces(bool),       // default: on
    LiteralSeparator(bool),  // default: off
    WordBoundary(bool),      // default: off
    Auto,
    Smart,
    Glob,
}

#[derive(Clone, Debug)]
pub struct CompiledFilter {
    token: Vec<CompiledFilterToken>,
    requires_lower_case: bool,
    requires_last_element: bool,
}

#[derive(Clone, Debug)]
enum CompiledFilterToken {
    Glob(GlobMatcher),
    SmartText(String),
    SmartNext(String),
    CaseSensitive,
    CaseInSensitive,    // default
    AnyOrder,           // default
    SameOrder,
    WholePath,          // default
    LastElement,
}

#[derive(Clone, Debug)]
struct Options {
    case_sensitive: bool,
    same_order: bool,
    last_element: bool,
    smart_spaces: bool,
    literal_separator: bool,
    word_boundaries: bool,
}

impl Options {
    fn new(config: &LocateConfig) -> Self {
        Options {
            case_sensitive: match config.case {
                crate::Case::MatchCase => true,
                crate::Case::IgnoreCase => false,
            },
            same_order: match config.order {
                crate::Order::AnyOrder => false,
                crate::Order::SameOrder => true,
            },
            last_element: match config.what {
                crate::What::WholePath => false,
                crate::What::LastElement => true,
            },
            smart_spaces: config.smart_spaces,
            literal_separator: config.literal_separator,
            word_boundaries: config.word_boundaries
        }
    }
}

pub fn compile(filter: &[FilterToken], config: &LocateConfig) -> Result<CompiledFilter, LocateError> {
    let mut options = Options::new(config);
    let mut compiled = CompiledFilter {
        token: Vec::new(),
        requires_lower_case: false,
        requires_last_element: false,
    };
    let mut mode: Mode = config.mode;
    for token in filter {
        match token {
            FilterToken::CaseSensitive   => { options.case_sensitive = true; compiled.token.push(CompiledFilterToken::CaseSensitive); },
            FilterToken::CaseInSensitive => { options.case_sensitive = false; compiled.token.push(CompiledFilterToken::CaseInSensitive); },
            FilterToken::Text(text) => {
                let mode = if mode == Mode::Auto {
                    if text.contains("*") { Mode::Glob }
                    else if text.contains("?") { Mode::Glob }
                    else if text.contains("[") { Mode::Glob }
                    else if text.contains("]") { Mode::Glob }
                    else { Mode::Plain }
                } else {
                    mode
                };
                if mode == Mode::Plain {
                    if options.case_sensitive {
                        if options.smart_spaces {
                            expand_smart_spaces(text.clone(), options.same_order, &mut compiled);
                        } else {
                            compiled.token.push(CompiledFilterToken::SmartText(text.clone()));
                        }
                    } else {
                        compiled.requires_lower_case = true;
                        if options.smart_spaces {
                            expand_smart_spaces(text.to_lowercase(), options.same_order, &mut compiled);
                        } else {
                            compiled.token.push(CompiledFilterToken::SmartText(text.to_lowercase()));
                        }
                    }
                }  
                else if mode == Mode::Glob {
                    let glob_matcher = GlobBuilder::new(text.as_str())
                        .case_insensitive(options.case_sensitive)
                        .literal_separator(options.literal_separator)
                        .backslash_escape(true)
                        .empty_alternates(true)
                        .build()
                        .map_err(|err| LocateError::GlobPatternError(text.clone(), err))?
                        .compile_matcher();
                    compiled.token.push(CompiledFilterToken::Glob(glob_matcher));
                };
                if options.last_element {
                    compiled.requires_last_element = true;
                }
            },
            FilterToken::AnyOrder => { options.same_order = false; compiled.token.push(CompiledFilterToken::AnyOrder); }
            FilterToken::SameOrder => { options.same_order = true; compiled.token.push(CompiledFilterToken::SameOrder); }
            FilterToken::WholePath => { options.last_element = false; compiled.token.push(CompiledFilterToken::WholePath); },
            FilterToken::LastElement => { options.last_element = true; compiled.token.push(CompiledFilterToken::LastElement); },
            FilterToken::SmartSpaces(on) => { options.smart_spaces = *on; },
            FilterToken::LiteralSeparator(on) => { options.literal_separator = *on; },
            FilterToken::WordBoundary(on) => { options.word_boundaries = *on; }
            FilterToken::Auto => { mode = Mode::Auto; },
            FilterToken::Smart => { mode = Mode::Plain; },
            FilterToken::Glob => {mode = Mode::Glob; },
        }
    }
    Ok(compiled)
}

fn expand_smart_spaces(text: String, mut b_same_order: bool, compiled: &mut CompiledFilter) {
    let mut first = true;
    let b_stored_same_order = b_same_order;
    for part in text.split(&[' ', '-', '_']) {
        if !part.is_empty() {
            if !first && !b_same_order {
                b_same_order = true;
                compiled.token.push(CompiledFilterToken::SameOrder);
            }
            if first {
                compiled.token.push(CompiledFilterToken::SmartText(part.to_string()));
                first = false;
            } else {
                compiled.token.push(CompiledFilterToken::SmartNext(part.to_string()));
            }    
        }
    }
    if !b_stored_same_order && b_same_order {
        compiled.token.push(CompiledFilterToken::AnyOrder);
    }
}

#[derive(Clone, Copy, Debug)]
struct State {
    filter_index: usize,
    pos: usize,             // actual or lower-case position in whole path or last element
}

struct Part<'a> {
    text: &'a str,
    offset: usize,
}


pub fn apply(text: &str, filter: &CompiledFilter, config: &LocateConfig) -> bool {
    let mut options = Options::new(config);
    let lower_text = if filter.requires_lower_case {
        Some(text.to_lowercase())
    } else {
        None
    };
    let (actual_last, lower_last) = if filter.requires_last_element {
        let a = if let Some(pos_last_slash) = text.rfind('/') {
            Some(Part {
                text: &text[pos_last_slash + 1..],
                offset: pos_last_slash + 1,
            })
        } else {
            Some(Part {
                text: &text[..],
                offset: 0,
            })
        };
        let b = if let Some(lower_text) = &lower_text {
            if let Some(pos_last_slash) = lower_text.rfind('/') {
                Some(Part {
                    text: &lower_text[pos_last_slash + 1..],
                    offset: pos_last_slash + 1,
                })
            } else {
                Some(Part {
                    text: &lower_text[..],
                    offset: 0,
                })
            }
        } else {
            None
        };
        (a, b)
    } else {
        (None, None)
    };
    let mut state = State {
        filter_index: 0,
        pos: 0,
    };
    let mut back_tracking = state;
    while state.filter_index < filter.token.len() {
        let token = &filter.token[state.filter_index];
        if let CompiledFilterToken::SmartText(_) = token {
            back_tracking = state;
        }
        state.filter_index = state.filter_index + 1;
        match token {
            CompiledFilterToken::SmartText(pattern) => {
                let (text, start) = match (options.last_element, options.same_order, options.case_sensitive) {
                    (true,  true,  true)  => (actual_last.as_ref().unwrap().text,    state.pos),
                    (true,  true,  false) => (lower_last.as_ref().unwrap().text,     state.pos),
                    (true,  false, true)  => (actual_last.as_ref().unwrap().text,    0),
                    (true,  false, false) => (lower_last.as_ref().unwrap().text,     0),
                    (false, true,  true)  => (text,                                  state.pos),
                    (false, true,  false) => (lower_text.as_ref().unwrap().as_str(), state.pos),
                    (false, false, true)  => (text,                                  0),
                    (false, false, false) => (lower_text.as_ref().unwrap().as_str(), 0),
                };
                if let Some(npos) = text[start..].find(pattern) {
                    state.pos = start + npos + pattern.len();
                } else {
                    return false;
                }
            },
            CompiledFilterToken::SmartNext(pattern) => {
                let text = match (options.last_element, options.same_order, options.case_sensitive) {
                    (true,  true,  true)  => actual_last.as_ref().unwrap().text,
                    (true,  true,  false) => lower_last.as_ref().unwrap().text,
                    (true,  false, true)  => actual_last.as_ref().unwrap().text,
                    (true,  false, false) => lower_last.as_ref().unwrap().text,
                    (false, true,  true)  => text,
                    (false, true,  false) => lower_text.as_ref().unwrap().as_str(),
                    (false, false, true)  => text,
                    (false, false, false) => lower_text.as_ref().unwrap().as_str(),
                };
                let skip = skip_separator(&text[state.pos..]);
                if text[state.pos+skip..].starts_with(pattern) {
                    state.pos = state.pos + skip + pattern.len();
                } else {
                    // Restore old state:
                    state = back_tracking;
                    // Consume one letter:
                    if let Some(ch) = text[state.pos..].chars().next() {
                        state.pos += ch.len_utf8();
                    }
                };
            },
            CompiledFilterToken::CaseSensitive => {
                if !options.case_sensitive {
                    // lower -> actual
                    options.case_sensitive = true;
                    if options.last_element {
                        if state.pos != 0 {
                            let pos = lower_last.as_ref().unwrap().text[0..state.pos].chars().count();
                            if let Some((new_pos, _ch)) = actual_last.as_ref().unwrap().text.char_indices().nth(pos) {
                                assert_eq!(new_pos, pos);
                                state.pos = new_pos;
                            }
                        }
                    } else {
                        if state.pos != 0 {
                            let pos = lower_text.as_ref().unwrap().as_str()[0..state.pos].chars().count();
                            if let Some((new_pos, _ch)) = text.char_indices().nth(pos) {
                                assert_eq!(new_pos, pos);
                                state.pos = new_pos;
                            }
                        }
                    }
                }
            },
            CompiledFilterToken::CaseInSensitive => {
                if options.case_sensitive {
                    // actual -> lower
                    options.case_sensitive = false;
                    if options.last_element {
                        if state.pos != 0 {
                            let pos = actual_last.as_ref().unwrap().text[0..state.pos].chars().count();
                            if let Some((new_pos, _ch)) = lower_last.as_ref().unwrap().text.char_indices().nth(pos) {
                                assert_eq!(new_pos, pos);
                                state.pos = new_pos;
                            }
                        }
                    } else {
                        if state.pos != 0 {
                            let pos = text[0..state.pos].chars().count();
                            if let Some((new_pos, _ch)) = lower_text.as_ref().unwrap().char_indices().nth(pos) {
                                assert_eq!(new_pos, pos);
                                state.pos = new_pos;
                            }
                        }
                    }
                }
            },
            CompiledFilterToken::AnyOrder => {
                options.same_order = false;
            },
            CompiledFilterToken::SameOrder => {
                options.same_order = true;
            },
            CompiledFilterToken::WholePath => {
                if  options.last_element {
                    let offset = if options.case_sensitive || !filter.requires_lower_case {
                        actual_last.as_ref().unwrap().offset
                    } else {
                        lower_last.as_ref().unwrap().offset
                    };
                    options.last_element = false;
                    state.pos = state.pos + offset;
                }
            }  
            CompiledFilterToken::LastElement => {
                if !options.last_element {
                    let offset = if options.case_sensitive || !filter.requires_lower_case {
                        actual_last.as_ref().unwrap().offset
                    } else {
                        lower_last.as_ref().unwrap().offset
                    };
                    options.last_element = true;
                    state.pos = if state.pos > offset { state.pos - offset} else { 0 };
                }
            },
            CompiledFilterToken::Glob(glob) => {
                if  options.last_element {
                    if !glob.is_match(actual_last.as_ref().unwrap().text) {
                        return false;
                    }
                } else {
                    if !glob.is_match(text) {
                        return false;
                    }
                }
            },
        }
    }
    true
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
        let config = LocateConfig::default();
        let flt = compile(flt, &config).unwrap();
        let config = LocateConfig::default();
        DATA.iter().filter(|entry: &&&str| apply(entry, &flt, &config)).map(|x: &&str| String::from(*x)).collect()
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
        let config = LocateConfig::default();
        assert_eq!(apply("foo bar", &compile(&[FilterToken::SameOrder, t("foo")], &config).unwrap(), &config), true);
        assert_eq!(apply("foo bar", &compile(&[FilterToken::SameOrder, t("foo"), t("foo")], &config).unwrap(), &config), false);
        assert_eq!(apply("foo bar baz", &compile(&[FilterToken::SameOrder, t("foo"), t("baz")], &config).unwrap(), &config), true);
        assert_eq!(apply("foo bar baz", &compile(&[t("foo baz")], &config).unwrap(), &config), false);
        assert_eq!(apply("fOO bar baZ", &compile(&[t("Foo Bar")], &config).unwrap(), &config), true);
        assert_eq!(apply("foo foo", &compile(&[t("foo foo")], &config).unwrap(), &config), true);
        assert_eq!(apply("foo bar", &compile(&[t("foo foo")], &config).unwrap(), &config), false);
        assert_eq!(apply("foo-foo", &compile(&[t("foo foo")], &config).unwrap(), &config), true);
        assert_eq!(apply("foo_foo", &compile(&[t("foo foo")], &config).unwrap(), &config), true);
    }

    #[test]
    fn smart_space() {
        let config = LocateConfig::default();
        assert_eq!(apply("foo bar abc baz", &compile(&[t("foo baz")], &config).unwrap(), &config), false);
        assert_eq!(apply("foo bar abc baz", &compile(&[t("bar abc")], &config).unwrap(), &config), true);
        assert_eq!(apply("foo-bar-abc-baz", &compile(&[t("bar abc")], &config).unwrap(), &config), true);
        assert_eq!(apply("foo_bar_abc_baz", &compile(&[t("bar abc")], &config).unwrap(), &config), true);
        assert_eq!(apply("foo_bar_abc_baz", &compile(&[t("bar-abc")], &config).unwrap(), &config), true);
        assert_eq!(apply("foo bar abc baz", &compile(&[t("bar_abc")], &config).unwrap(), &config), true);
    }

    #[test]
    fn retry_on_failure_with_next() {
        let config = LocateConfig::default();
        assert_eq!(apply("foo bar baz", &compile(&[t("b-a-r")], &config).unwrap(), &config), true);
        assert_eq!(apply("foo baz bar", &compile(&[t("b-a-r")], &config).unwrap(), &config), true);
        assert_eq!(apply("foo baz bax", &compile(&[t("b-a-r")], &config).unwrap(), &config), false);
    }

    #[test]
    fn back_tracking_skip_multibyte_characters() {
        let config = LocateConfig::default();
        let text = "äaäa";   //  [61, CC, 88, 61, C3, A4, 61]
        // 0x61      : a
        // 0xCC, 0x88: Trema for previous letter (https://www.compart.com/de/unicode/U+0308)
        // 0xC3, 0xA4: ä
        // println!("{:02X?}", text.bytes());
        assert_eq!(apply(text, &compile(&[t("a-b")], &config).unwrap(), &config), false);
    }

    #[test]
    fn position_calculation_same_order() {
        let config = LocateConfig::default();
        let text = "              a            bc";
        for a in &[CompiledFilterToken::CaseInSensitive, CompiledFilterToken::CaseInSensitive] {
            for b in &[CompiledFilterToken::WholePath, CompiledFilterToken::LastElement] {
                assert_eq!(apply(text, &CompiledFilter {
                    token: vec![
                        a.clone(),
                        b.clone(),
                        CompiledFilterToken::SameOrder,
                        CompiledFilterToken::SmartText("a".to_string()),
                        CompiledFilterToken::SmartText("b".to_string()),
                        CompiledFilterToken::SmartNext("c".to_string())
                    ],
                    requires_last_element: true,
                    requires_lower_case: true,
                }, &config),
                true);
            }
        }
    }

    #[test]
    fn position_calculation_any_order() {
        let config = LocateConfig::default();
        let text = "              bc            a";
        for a in &[CompiledFilterToken::CaseInSensitive, CompiledFilterToken::CaseInSensitive] {
            for b in &[CompiledFilterToken::WholePath, CompiledFilterToken::LastElement] {
                assert_eq!(apply(text, &CompiledFilter {
                    token: vec![
                        a.clone(),
                        b.clone(),
                        CompiledFilterToken::AnyOrder,
                        CompiledFilterToken::SmartText("a".to_string()),
                        CompiledFilterToken::SmartText("b".to_string()),
                        CompiledFilterToken::SmartNext("c".to_string())
                    ],
                    requires_last_element: true,
                    requires_lower_case: true,
                }, &config),
                true);
            }
        }
    }

    #[test]
    fn compile_text_with_spaces() {
        let config = LocateConfig::default();
        let actual = compile( &[
            t("a b c d"),
            t("e"),
        ], &config).unwrap();
        let expected = CompiledFilter {
            token: vec![
                CompiledFilterToken::SmartText("a".to_string()),
                CompiledFilterToken::SameOrder,
                CompiledFilterToken::SmartNext("b".to_string()),
                CompiledFilterToken::SmartNext("c".to_string()),
                CompiledFilterToken::SmartNext("d".to_string()),
                CompiledFilterToken::AnyOrder,
                CompiledFilterToken::SmartText("e".to_string()),
            ],
            requires_last_element: true,
            requires_lower_case: true,
        };
        // Can't use assert_eq! here, since PartialEq is not implemented for GlobMatcher.
        check_compiled_filter(actual, expected);
    }

    #[test]
    fn remove_empty_strings_after_expanding_smart_spaces() {
        let config = LocateConfig::default();
        let actual = compile(&[t("- a-b c- -d -")], &config).unwrap();
        let expected = CompiledFilter {
            token: vec![
                CompiledFilterToken::SmartText("a".to_string()),
                CompiledFilterToken::SameOrder,
                CompiledFilterToken::SmartNext("b".to_string()),
                CompiledFilterToken::SmartNext("c".to_string()),
                CompiledFilterToken::SmartNext("d".to_string()),
                CompiledFilterToken::AnyOrder,
            ],
            requires_last_element: true,
            requires_lower_case: true,
        };
        check_compiled_filter(actual, expected);
    }

    fn check_compiled_filter(actual: CompiledFilter, expected: CompiledFilter) {
        assert_eq!(actual.token.len(), expected.token.len());
        for (idx, (a,b)) in expected.token.iter().zip(actual.token.iter()).enumerate() {
            let ok = match (a, b) {
                (CompiledFilterToken::Glob(a), CompiledFilterToken::Glob(b)) => a.glob() == b.glob(),
                (CompiledFilterToken::SmartText(a), CompiledFilterToken::SmartText(b)) => a == b,
                (CompiledFilterToken::SmartNext(a), CompiledFilterToken::SmartNext(b)) => a == b,
                (CompiledFilterToken::CaseSensitive, CompiledFilterToken::CaseSensitive) => true,
                (CompiledFilterToken::CaseInSensitive, CompiledFilterToken::CaseInSensitive) => true,
                (CompiledFilterToken::AnyOrder, CompiledFilterToken::AnyOrder) => true,
                (CompiledFilterToken::SameOrder, CompiledFilterToken::SameOrder) => true,
                (CompiledFilterToken::WholePath, CompiledFilterToken::WholePath) => true,
                (CompiledFilterToken::LastElement, CompiledFilterToken::LastElement) => true,
                (_, _) => false,
            };
            assert!(ok, "Element {idx} not as expected: {a:?} != {b:?}");
        }
    }

    #[test]
    fn glob_star() {
        assert_eq!(process(&[FilterToken::Glob, t("*i")]), [S2, S3]);
    }

    #[test]
    fn glob_recursive_wildcard() {
        assert_eq!(process(&[FilterToken::Glob,  FilterToken::LiteralSeparator(false), t("/**/*s")]), [S1]);
    }

    #[test]
    fn glob_question_mark() {
        assert_eq!(process(&[FilterToken::Glob, t("*/???i")]), [S2, S3]);
    }

    #[test]
    fn glob_require_literal_separator() {
        assert_eq!(process(&[FilterToken::Glob, FilterToken::LiteralSeparator(false), t("/*i")]), [S2, S3]);
        assert_eq!(process(&[FilterToken::Glob, FilterToken::LiteralSeparator(true), t("/*i")]), EMPTY);
        assert_eq!(process(&[FilterToken::Glob, FilterToken::LiteralSeparator(true), t("/*/*/*/*i")]), [S2, S3]);
        assert_eq!(process(&[FilterToken::Glob, FilterToken::LiteralSeparator(true), t("/**/*i")]), [S2, S3]);
        assert_eq!(process(&[FilterToken::Glob, FilterToken::LiteralSeparator(true), t("/**/eins")]), [S1]);
    }

    #[test]
    fn switching_between_whole_path_and_last_element_position_modes() {
        assert_eq!(process(&[FilterToken::SameOrder, FilterToken::LastElement, t("z"), FilterToken::WholePath, t("wei")]), [S2]);
        assert_eq!(process(&[FilterToken::SameOrder, FilterToken::LastElement, t("z"), FilterToken::WholePath, t("x")]), EMPTY);
        assert_eq!(process(&[FilterToken::SameOrder, FilterToken::WholePath, t("x"), FilterToken::LastElement, t("zwei")]), [S2]);
        assert_eq!(process(&[FilterToken::SameOrder, FilterToken::WholePath, t("zw"), FilterToken::LastElement, t("ei")]), [S2]);
        assert_eq!(process(&[FilterToken::SameOrder, FilterToken::WholePath, t("zwe"), FilterToken::LastElement, t("ei")]), EMPTY);
    }

    #[test]
    fn glob_on_last_element_only() {
        assert_eq!(process(&[FilterToken::Glob, FilterToken::LastElement, t("*.txt")]), [S7]);
        assert_eq!(process(&[FilterToken::Glob, FilterToken::WholePath, t("*.txt")]), [S7]);
        assert_eq!(process(&[FilterToken::Glob, FilterToken::WholePath, FilterToken::LiteralSeparator(true), t("*.txt")]), EMPTY);
    }

    #[test]
    fn utf8_slice() {
        let text = "öäüÄÖÜß";
        assert_eq!(text.len(), 14);
        assert_eq!(text.chars().count(), 7);
        let ch = text[0..].chars().next().unwrap();
        let len = ch.len_utf8();
        assert_eq!(len, 2);
        assert_eq!(text[2..].chars().next().unwrap(), 'ä');
    }
}
