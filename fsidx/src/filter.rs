use crate::config::{LocateConfig, Mode};
use crate::find::FindExt;
use crate::locate::LocateError;
use globset::{GlobBuilder, GlobMatcher};

#[derive(Clone, Debug, PartialEq)]
pub enum FilterToken {
    Text(String),
    CaseSensitive,
    CaseInSensitive, // default
    AnyOrder,        // default
    SameOrder,
    WholePath, // default
    LastElement,
    SmartSpaces(bool),      // default: on
    LiteralSeparator(bool), // default: off
    WordBoundary(bool),     // default: off
    Auto,
    Smart,
    Glob,
}

#[derive(Clone, Debug)]
pub struct CompiledFilter {
    token: Vec<CompiledFilterToken>,
}

#[derive(Clone, Debug)]
enum CompiledFilterToken {
    GoToStart,
    GoToLastElement,
    EnsureLastElement,
    Glob(GlobMatcher, bool),
    FindCaseInsensitive(String),
    FindCaseSensitive(String),
    FindWordStartBoundary,
    SkipSmartSpace,
    ExpectCaseInsensitive(String),
    ExpectCaseSensitive(String),
    ExpectWordEndBoundary,
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
            case_sensitive: config.case_sensitive,
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
            word_boundaries: config.word_boundaries,
        }
    }
}

pub fn compile(
    filter: &[FilterToken],
    config: &LocateConfig,
) -> Result<CompiledFilter, LocateError> {
    let mut options = Options::new(config);
    let mut compiled = CompiledFilter { token: Vec::new() };
    let mut mode: Mode = config.mode;
    let mut nothing = true;
    for token in filter {
        match token {
            FilterToken::CaseSensitive => {
                options.case_sensitive = true;
            }
            FilterToken::CaseInSensitive => {
                options.case_sensitive = false;
            }
            FilterToken::Text(text) => {
                let mode = if mode == Mode::Auto {
                    if text.contains(['*', '?', '[', ']', '{', '}']) {
                        Mode::Glob
                    } else {
                        Mode::Plain
                    }
                } else {
                    mode
                };
                if mode == Mode::Plain {
                    if options.same_order {
                        if options.last_element {
                            compiled.token.push(CompiledFilterToken::EnsureLastElement);
                        }
                    } else if options.last_element {
                        compiled.token.push(CompiledFilterToken::GoToLastElement);
                    } else {
                        compiled.token.push(CompiledFilterToken::GoToStart);
                    }
                    let fragments: Vec<String> = if options.smart_spaces {
                        text.split(&[' ', '-', '_'])
                            .filter(|s| !s.is_empty())
                            .map(str::to_string)
                            .collect()
                    } else {
                        vec![text.clone()]
                    };
                    let mut it = fragments.into_iter();
                    if let Some(fragment) = it.next() {
                        if options.word_boundaries {
                            compiled
                                .token
                                .push(CompiledFilterToken::FindWordStartBoundary);
                            if options.case_sensitive {
                                compiled
                                    .token
                                    .push(CompiledFilterToken::ExpectCaseSensitive(fragment));
                            } else {
                                compiled
                                    .token
                                    .push(CompiledFilterToken::ExpectCaseInsensitive(
                                        fragment.to_uppercase(),
                                    ));
                            }
                        } else if options.case_sensitive {
                            compiled
                                .token
                                .push(CompiledFilterToken::FindCaseSensitive(fragment));
                        } else {
                            compiled
                                .token
                                .push(CompiledFilterToken::FindCaseInsensitive(
                                    fragment.to_uppercase(),
                                ));
                        }
                        nothing = false;
                    }
                    for fragment in it {
                        compiled.token.push(CompiledFilterToken::SkipSmartSpace);
                        if options.case_sensitive {
                            compiled
                                .token
                                .push(CompiledFilterToken::ExpectCaseSensitive(fragment));
                        } else {
                            compiled
                                .token
                                .push(CompiledFilterToken::ExpectCaseInsensitive(
                                    fragment.to_uppercase(),
                                ));
                        }
                    }
                    if options.word_boundaries {
                        compiled
                            .token
                            .push(CompiledFilterToken::ExpectWordEndBoundary);
                    }
                } else if mode == Mode::Glob {
                    if options.last_element {
                        compiled.token.push(CompiledFilterToken::GoToLastElement);
                    }
                    let glob_matcher = GlobBuilder::new(text.as_str())
                        .case_insensitive(options.case_sensitive)
                        .literal_separator(options.literal_separator)
                        .backslash_escape(true)
                        .empty_alternates(true)
                        .build()
                        .map_err(|err| LocateError::GlobPatternError(text.clone(), err))?
                        .compile_matcher();
                    compiled.token.push(CompiledFilterToken::Glob(
                        glob_matcher,
                        options.last_element,
                    ));
                    nothing = false;
                };
            }
            FilterToken::AnyOrder => {
                options.same_order = false;
            }
            FilterToken::SameOrder => {
                options.same_order = true;
            }
            FilterToken::WholePath => {
                options.last_element = false;
            }
            FilterToken::LastElement => {
                options.last_element = true;
            }
            FilterToken::SmartSpaces(on) => {
                options.smart_spaces = *on;
            }
            FilterToken::LiteralSeparator(on) => {
                options.literal_separator = *on;
            }
            FilterToken::WordBoundary(on) => {
                options.word_boundaries = *on;
            }
            FilterToken::Auto => {
                mode = Mode::Auto;
            }
            FilterToken::Smart => {
                mode = Mode::Plain;
            }
            FilterToken::Glob => {
                mode = Mode::Glob;
            }
        }
    }
    if nothing {
        return Err(LocateError::Trivial);
    }
    Ok(compiled)
}

#[derive(Clone, Copy, Debug)]
struct State {
    filter_index: usize,
    pos: usize, // actual or lower-case position in whole path or last element
}

pub fn apply(text: &str, filter: &CompiledFilter) -> bool {
    let mut pos_last: Option<usize> = None;
    let mut state = State {
        filter_index: 0,
        pos: 0,
    };
    let mut back_tracking = state;
    while state.filter_index < filter.token.len() {
        let token = &filter.token[state.filter_index];
        if let CompiledFilterToken::FindCaseInsensitive(_) = token {
            back_tracking = state;
        } else if let CompiledFilterToken::FindCaseSensitive(_) = token {
            back_tracking = state;
        } else if let CompiledFilterToken::FindWordStartBoundary = token {
            back_tracking = state;
        }
        state.filter_index += 1;
        match token {
            CompiledFilterToken::GoToStart => {
                state.pos = 0;
            }
            CompiledFilterToken::GoToLastElement => {
                if pos_last.is_none() {
                    pos_last = Some(if let Some(pos_last) = text.rfind('/') {
                        pos_last + 1
                    } else {
                        0
                    });
                }
                state.pos = pos_last.unwrap();
            }
            CompiledFilterToken::EnsureLastElement => {
                if pos_last.is_none() {
                    pos_last = Some(if let Some(pos_last) = text.rfind('/') {
                        pos_last + 1
                    } else {
                        0
                    });
                }
                if state.pos < pos_last.unwrap() {
                    state.pos = pos_last.unwrap();
                }
            }
            CompiledFilterToken::Glob(glob, last_element) => {
                let text = if *last_element {
                    if pos_last.is_none() {
                        pos_last = Some(if let Some(pos_last) = text.rfind('/') {
                            pos_last + 1
                        } else {
                            0
                        });
                    }
                    &text[pos_last.unwrap()..]
                } else {
                    text
                };
                if !glob.is_match(text) {
                    return false;
                };
            }
            CompiledFilterToken::FindCaseInsensitive(pattern) => {
                if let Some(range) = text.find_case_insensitive(state.pos, pattern) {
                    state.pos = range.end;
                } else {
                    return false;
                }
            }
            CompiledFilterToken::FindCaseSensitive(pattern) => {
                if let Some(range) = text.find_case_sensitive(state.pos, pattern) {
                    state.pos = range.end;
                } else {
                    return false;
                }
            }
            CompiledFilterToken::FindWordStartBoundary => {
                if let Some(pos) = text.find_word_start_boundary(state.pos) {
                    state.pos = pos;
                } else {
                    return false;
                }
            }
            CompiledFilterToken::SkipSmartSpace => {
                state.pos = text.skip_smart_space(state.pos);
            }
            CompiledFilterToken::ExpectCaseInsensitive(pattern) => {
                if let Some(range) = text.tag_case_insensitive(state.pos, pattern) {
                    state.pos = range.end;
                } else {
                    state = State {
                        filter_index: back_tracking.filter_index,
                        pos: text.skip_character(back_tracking.pos),
                    };
                }
            }
            CompiledFilterToken::ExpectCaseSensitive(pattern) => {
                if let Some(range) = text.tag_case_sensitive(state.pos, pattern) {
                    state.pos = range.end;
                } else {
                    state = State {
                        filter_index: back_tracking.filter_index,
                        pos: text.skip_character(back_tracking.pos),
                    };
                }
            }
            CompiledFilterToken::ExpectWordEndBoundary => {
                if !text.tag_word_end_boundary(state.pos) {
                    state = State {
                        filter_index: back_tracking.filter_index,
                        pos: text.skip_character(back_tracking.pos),
                    };
                }
            }
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
    static S7: &str = "/path/to/hidden/.file.txt";

    static DATA: [&str; 8] = [S0, S1, S2, S3, S4, S5, S6, S7];

    fn process(flt: &[FilterToken]) -> Vec<String> {
        let config = LocateConfig::default();
        let flt = compile(flt, &config).unwrap();
        DATA.iter()
            .filter(|entry: &&&str| apply(entry, &flt))
            .map(|x: &&str| String::from(*x))
            .collect()
    }

    static EMPTY: [&str; 0] = [];
    fn t(s: &str) -> FilterToken {
        FilterToken::Text(String::from(s))
    }

    #[test]
    fn nothing_with_empty_string() {
        let config = LocateConfig::default();
        assert!(matches!(
            compile(&[t("")], &config),
            Err(LocateError::Trivial)
        ));
    }

    #[test]
    fn nothing_with_empty_list() {
        let config = LocateConfig::default();
        assert!(matches!(compile(&[], &config), Err(LocateError::Trivial)));
    }

    #[test]
    fn default() {
        assert_eq!(process(&[t("Y"), t("G"), t("A")]), [S1, S2, S3, S4]);
    }

    #[test]
    fn case_insensitive_any_order_whole_path() {
        assert_eq!(
            process(&[
                FilterToken::CaseInSensitive,
                FilterToken::AnyOrder,
                FilterToken::WholePath,
                t("Y"),
                t("A"),
                t("G")
            ]),
            [S1, S2, S3, S4]
        );
        assert_eq!(
            process(&[
                FilterToken::CaseInSensitive,
                FilterToken::AnyOrder,
                FilterToken::WholePath,
                t("a"),
                t("a"),
                t("g")
            ]),
            [S1, S2, S3, S4]
        );
    }

    #[test]
    fn case_sensitive_any_order_whole_path() {
        assert_eq!(
            process(&[
                FilterToken::CaseSensitive,
                FilterToken::AnyOrder,
                FilterToken::WholePath,
                t("Y"),
                t("A"),
                t("G")
            ]),
            [S1, S3, S4]
        );
        assert_eq!(
            process(&[
                FilterToken::CaseSensitive,
                FilterToken::AnyOrder,
                FilterToken::WholePath,
                t("y"),
                t("A"),
                t("G")
            ]),
            EMPTY
        );
    }

    #[test]
    fn case_insensitive_same_order_whole_path() {
        assert_eq!(
            process(&[
                FilterToken::CaseInSensitive,
                FilterToken::SameOrder,
                FilterToken::WholePath,
                t("Y"),
                t("A"),
                t("G")
            ]),
            [S4]
        );
        assert_eq!(
            process(&[
                FilterToken::CaseInSensitive,
                FilterToken::SameOrder,
                FilterToken::WholePath,
                t("y"),
                t("a"),
                t("g")
            ]),
            [S4]
        );
    }

    #[test]
    fn case_sensitive_same_order_whole_path() {
        assert_eq!(
            process(&[
                FilterToken::CaseSensitive,
                FilterToken::SameOrder,
                FilterToken::WholePath,
                t("Y"),
                t("A"),
                t("G")
            ]),
            [S4]
        );
        assert_eq!(
            process(&[
                FilterToken::CaseSensitive,
                FilterToken::SameOrder,
                FilterToken::WholePath,
                t("Y"),
                t("a"),
                t("G")
            ]),
            EMPTY
        );
    }

    #[test]
    fn case_insensitive_any_order_last_component() {
        assert_eq!(
            process(&[
                FilterToken::CaseInSensitive,
                FilterToken::AnyOrder,
                FilterToken::LastElement,
                t("e"),
                t("d")
            ]),
            [S0, S3]
        );
        assert_eq!(
            process(&[
                FilterToken::CaseInSensitive,
                FilterToken::AnyOrder,
                FilterToken::LastElement,
                t("E"),
                t("d")
            ]),
            [S0, S3]
        );
    }

    #[test]
    fn case_sensitive_any_order_last_component() {
        assert_eq!(
            process(&[
                FilterToken::CaseSensitive,
                FilterToken::AnyOrder,
                FilterToken::LastElement,
                t("e"),
                t("d")
            ]),
            [S3]
        );
        assert_eq!(
            process(&[
                FilterToken::CaseSensitive,
                FilterToken::AnyOrder,
                FilterToken::LastElement,
                t("E"),
                t("D")
            ]),
            [S0]
        );
    }

    #[test]
    fn case_insensitive_same_order_last_component() {
        assert_eq!(
            process(&[
                FilterToken::CaseInSensitive,
                FilterToken::SameOrder,
                FilterToken::LastElement,
                t("e"),
                t("d")
            ]),
            EMPTY
        );
        assert_eq!(
            process(&[
                FilterToken::CaseInSensitive,
                FilterToken::SameOrder,
                FilterToken::LastElement,
                t("d"),
                t("e")
            ]),
            [S0, S3]
        );
        assert_eq!(
            process(&[
                FilterToken::CaseInSensitive,
                FilterToken::SameOrder,
                FilterToken::LastElement,
                t("d"),
                t("E")
            ]),
            [S0, S3]
        );
    }

    #[test]
    fn case_sensitive_same_order_last_component() {
        assert_eq!(
            process(&[
                FilterToken::CaseSensitive,
                FilterToken::SameOrder,
                FilterToken::LastElement,
                t("e"),
                t("d")
            ]),
            EMPTY
        );
        assert_eq!(
            process(&[
                FilterToken::CaseSensitive,
                FilterToken::SameOrder,
                FilterToken::LastElement,
                t("d"),
                t("e")
            ]),
            [S3]
        );
        assert_eq!(
            process(&[
                FilterToken::CaseSensitive,
                FilterToken::SameOrder,
                FilterToken::LastElement,
                t("e"),
                t("d")
            ]),
            EMPTY
        );
        assert_eq!(
            process(&[
                FilterToken::CaseSensitive,
                FilterToken::SameOrder,
                FilterToken::LastElement,
                t("D"),
                t("E")
            ]),
            [S0]
        );
        assert_eq!(
            process(&[
                FilterToken::CaseSensitive,
                FilterToken::SameOrder,
                FilterToken::LastElement,
                t("E"),
                t("D")
            ]),
            EMPTY
        );
    }

    #[test]
    fn continue_after_last_match() {
        let config = LocateConfig::default();
        assert_eq!(
            apply(
                "foo bar",
                &compile(&[FilterToken::SameOrder, t("foo")], &config).unwrap()
            ),
            true
        );
        assert_eq!(
            apply(
                "foo bar",
                &compile(&[FilterToken::SameOrder, t("foo"), t("foo")], &config).unwrap()
            ),
            false
        );
        assert_eq!(
            apply(
                "foo bar baz",
                &compile(&[FilterToken::SameOrder, t("foo"), t("baz")], &config).unwrap()
            ),
            true
        );
        assert_eq!(
            apply("foo bar baz", &compile(&[t("foo baz")], &config).unwrap()),
            false
        );
        assert_eq!(
            apply("fOO bar baZ", &compile(&[t("Foo Bar")], &config).unwrap()),
            true
        );
        assert_eq!(
            apply("foo foo", &compile(&[t("foo foo")], &config).unwrap()),
            true
        );
        assert_eq!(
            apply("foo bar", &compile(&[t("foo foo")], &config).unwrap()),
            false
        );
        assert_eq!(
            apply("foo-foo", &compile(&[t("foo foo")], &config).unwrap()),
            true
        );
        assert_eq!(
            apply("foo_foo", &compile(&[t("foo foo")], &config).unwrap()),
            true
        );
    }

    #[test]
    fn smart_space() {
        let config = LocateConfig::default();
        assert_eq!(
            apply(
                "foo bar abc baz",
                &compile(&[t("foo baz")], &config).unwrap()
            ),
            false
        );
        assert_eq!(
            apply(
                "foo bar abc baz",
                &compile(&[t("bar abc")], &config).unwrap()
            ),
            true
        );
        assert_eq!(
            apply(
                "foo-bar-abc-baz",
                &compile(&[t("bar abc")], &config).unwrap()
            ),
            true
        );
        assert_eq!(
            apply(
                "foo_bar_abc_baz",
                &compile(&[t("bar abc")], &config).unwrap()
            ),
            true
        );
        assert_eq!(
            apply(
                "foo_bar_abc_baz",
                &compile(&[t("bar-abc")], &config).unwrap()
            ),
            true
        );
        assert_eq!(
            apply(
                "foo bar abc baz",
                &compile(&[t("bar_abc")], &config).unwrap()
            ),
            true
        );
    }

    #[test]
    fn retry_on_failure_with_next() {
        let config = LocateConfig::default();
        assert_eq!(
            apply("foo bar baz", &compile(&[t("b-a-r")], &config).unwrap()),
            true
        );
        assert_eq!(
            apply("foo baz bar", &compile(&[t("b-a-r")], &config).unwrap()),
            true
        );
        assert_eq!(
            apply("foo baz bax", &compile(&[t("b-a-r")], &config).unwrap()),
            false
        );
    }

    #[test]
    fn back_tracking_skip_multibyte_characters() {
        let config = LocateConfig::default();
        let text = "äaäa"; //  [61, CC, 88, 61, C3, A4, 61]
                           // 0x61      : a
                           // 0xCC, 0x88: Trema for previous letter (https://www.compart.com/de/unicode/U+0308)
                           // 0xC3, 0xA4: ä
                           // println!("{:02X?}", text.bytes());
        assert_eq!(apply(text, &compile(&[t("a-b")], &config).unwrap()), false);
    }

    #[test]
    fn compile_text_with_spaces() {
        let config = LocateConfig::default();
        let actual = compile(&[t("a b c d"), t("e")], &config).unwrap();
        let expected = CompiledFilter {
            token: vec![
                CompiledFilterToken::GoToStart,
                CompiledFilterToken::FindCaseInsensitive("A".to_string()),
                CompiledFilterToken::SkipSmartSpace,
                CompiledFilterToken::ExpectCaseInsensitive("B".to_string()),
                CompiledFilterToken::SkipSmartSpace,
                CompiledFilterToken::ExpectCaseInsensitive("C".to_string()),
                CompiledFilterToken::SkipSmartSpace,
                CompiledFilterToken::ExpectCaseInsensitive("D".to_string()),
                CompiledFilterToken::GoToStart,
                CompiledFilterToken::FindCaseInsensitive("E".to_string()),
            ],
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
                CompiledFilterToken::GoToStart,
                CompiledFilterToken::FindCaseInsensitive("A".to_string()),
                CompiledFilterToken::SkipSmartSpace,
                CompiledFilterToken::ExpectCaseInsensitive("B".to_string()),
                CompiledFilterToken::SkipSmartSpace,
                CompiledFilterToken::ExpectCaseInsensitive("C".to_string()),
                CompiledFilterToken::SkipSmartSpace,
                CompiledFilterToken::ExpectCaseInsensitive("D".to_string()),
            ],
        };
        check_compiled_filter(actual, expected);
    }

    fn check_compiled_filter(actual: CompiledFilter, expected: CompiledFilter) {
        assert_eq!(actual.token.len(), expected.token.len());
        for (idx, (a, b)) in expected.token.iter().zip(actual.token.iter()).enumerate() {
            let ok = match (a, b) {
                (CompiledFilterToken::GoToStart, CompiledFilterToken::GoToStart) => true,
                (CompiledFilterToken::GoToLastElement, CompiledFilterToken::GoToLastElement) => {
                    true
                }
                (CompiledFilterToken::Glob(a1, a2), CompiledFilterToken::Glob(b1, b2)) => {
                    a1.glob() == b1.glob() && a2 == b2
                }
                (
                    CompiledFilterToken::FindCaseInsensitive(a),
                    CompiledFilterToken::FindCaseInsensitive(b),
                ) => a == b,
                (
                    CompiledFilterToken::FindCaseSensitive(a),
                    CompiledFilterToken::FindCaseSensitive(b),
                ) => a == b,
                (
                    CompiledFilterToken::FindWordStartBoundary,
                    CompiledFilterToken::FindWordStartBoundary,
                ) => true,
                (CompiledFilterToken::SkipSmartSpace, CompiledFilterToken::SkipSmartSpace) => true,
                (
                    CompiledFilterToken::ExpectCaseInsensitive(a),
                    CompiledFilterToken::ExpectCaseInsensitive(b),
                ) => a == b,
                (
                    CompiledFilterToken::ExpectCaseSensitive(a),
                    CompiledFilterToken::ExpectCaseSensitive(b),
                ) => a == b,
                (
                    CompiledFilterToken::ExpectWordEndBoundary,
                    CompiledFilterToken::ExpectWordEndBoundary,
                ) => true,
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
        assert_eq!(
            process(&[
                FilterToken::Glob,
                FilterToken::LiteralSeparator(false),
                t("/**/*s")
            ]),
            [S1]
        );
    }

    #[test]
    fn glob_question_mark() {
        assert_eq!(process(&[FilterToken::Glob, t("*/???i")]), [S2, S3]);
    }

    #[test]
    fn glob_require_literal_separator() {
        assert_eq!(
            process(&[
                FilterToken::Glob,
                FilterToken::LiteralSeparator(false),
                t("/*i")
            ]),
            [S2, S3]
        );
        assert_eq!(
            process(&[
                FilterToken::Glob,
                FilterToken::LiteralSeparator(true),
                t("/*i")
            ]),
            EMPTY
        );
        assert_eq!(
            process(&[
                FilterToken::Glob,
                FilterToken::LiteralSeparator(true),
                t("/*/*/*/*i")
            ]),
            [S2, S3]
        );
        assert_eq!(
            process(&[
                FilterToken::Glob,
                FilterToken::LiteralSeparator(true),
                t("/**/*i")
            ]),
            [S2, S3]
        );
        assert_eq!(
            process(&[
                FilterToken::Glob,
                FilterToken::LiteralSeparator(true),
                t("/**/eins")
            ]),
            [S1]
        );
    }

    #[test]
    fn switching_between_whole_path_and_last_element_position_modes() {
        assert_eq!(
            process(&[
                FilterToken::SameOrder,
                FilterToken::LastElement,
                t("z"),
                FilterToken::WholePath,
                t("wei")
            ]),
            [S2]
        );
        assert_eq!(
            process(&[
                FilterToken::SameOrder,
                FilterToken::LastElement,
                t("z"),
                FilterToken::WholePath,
                t("x")
            ]),
            EMPTY
        );
        assert_eq!(
            process(&[
                FilterToken::SameOrder,
                FilterToken::WholePath,
                t("x"),
                FilterToken::LastElement,
                t("zwei")
            ]),
            [S2]
        );
        assert_eq!(
            process(&[
                FilterToken::SameOrder,
                FilterToken::WholePath,
                t("zw"),
                FilterToken::LastElement,
                t("ei")
            ]),
            [S2]
        );
        assert_eq!(
            process(&[
                FilterToken::SameOrder,
                FilterToken::WholePath,
                t("zwe"),
                FilterToken::LastElement,
                t("ei")
            ]),
            EMPTY
        );
    }

    #[test]
    fn glob_on_last_element_only() {
        assert_eq!(
            process(&[FilterToken::Glob, FilterToken::LastElement, t("*.txt")]),
            [S7]
        );
        assert_eq!(
            process(&[FilterToken::Glob, FilterToken::WholePath, t("*.txt")]),
            [S7]
        );
        assert_eq!(
            process(&[
                FilterToken::Glob,
                FilterToken::WholePath,
                FilterToken::LiteralSeparator(true),
                t("*.txt")
            ]),
            EMPTY
        );
    }

    #[test]
    fn test_word_boundary() {
        let config = LocateConfig::default();
        assert_eq!(
            apply(
                "foobar",
                &compile(&[FilterToken::WordBoundary(true), t("foo")], &config).unwrap()
            ),
            false
        );
        assert_eq!(
            apply(
                "foobar",
                &compile(&[FilterToken::WordBoundary(true), t("bar")], &config).unwrap()
            ),
            false
        );
        assert_eq!(
            apply(
                "foo bar",
                &compile(&[FilterToken::WordBoundary(true), t("foo")], &config).unwrap()
            ),
            true
        );
        assert_eq!(
            apply(
                "foo bar",
                &compile(&[FilterToken::WordBoundary(true), t("bar")], &config).unwrap()
            ),
            true
        );
        assert_eq!(
            apply(
                "foo bar baz",
                &compile(&[FilterToken::WordBoundary(true), t("bar")], &config).unwrap()
            ),
            true
        );
        assert_eq!(
            apply(
                "foo bar baz",
                &compile(&[FilterToken::WordBoundary(true), t("ar")], &config).unwrap()
            ),
            false
        );
        assert_eq!(
            apply(
                "foo bar baz",
                &compile(&[FilterToken::WordBoundary(true), t("ba")], &config).unwrap()
            ),
            false
        );
        assert_eq!(
            apply(
                "FooBarBaz",
                &compile(&[FilterToken::WordBoundary(true), t("Bar")], &config).unwrap()
            ),
            true
        );
        assert_eq!(
            apply(
                "FooBarBaz",
                &compile(&[FilterToken::WordBoundary(true), t("Ba")], &config).unwrap()
            ),
            false
        );
        assert_eq!(
            apply(
                "FooBarBaz",
                &compile(&[FilterToken::WordBoundary(true), t("ar")], &config).unwrap()
            ),
            false
        );
        assert_eq!(
            apply(
                "abc123def456",
                &compile(&[FilterToken::WordBoundary(true), t("123")], &config).unwrap()
            ),
            true
        );
        assert_eq!(
            apply(
                "abc123def456",
                &compile(&[FilterToken::WordBoundary(true), t("def")], &config).unwrap()
            ),
            true
        );
        assert_eq!(
            apply(
                "abc123def456",
                &compile(&[FilterToken::WordBoundary(true), t("12")], &config).unwrap()
            ),
            false
        );
        assert_eq!(
            apply(
                "abc123def456",
                &compile(&[FilterToken::WordBoundary(true), t("23")], &config).unwrap()
            ),
            false
        );
        assert_eq!(
            apply(
                "abc123def456",
                &compile(&[FilterToken::WordBoundary(true), t("de")], &config).unwrap()
            ),
            false
        );
        assert_eq!(
            apply(
                "abc123def456",
                &compile(&[FilterToken::WordBoundary(true), t("ef")], &config).unwrap()
            ),
            false
        );
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
