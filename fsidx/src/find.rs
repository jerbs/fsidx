use std::ops::Range;

pub trait FindExt {
    fn find_case_sensitive(&self, start: usize, pattern: &str) -> Option<Range<usize>>;
    fn find_case_insensitive(&self, start: usize, pattern: &str) -> Option<Range<usize>>;
    fn skip_character(&self, start: usize) -> usize;
    fn skip_smart_space(&self, start: usize) -> usize;
    fn tag_case_sensitive(&self, start: usize, pattern: &str) -> Option<Range<usize>>;
    fn tag_case_insensitive(&self, start: usize, pattern: &str) -> Option<Range<usize>>;
    fn find_word_start_boundary(&self, start: usize) -> Option<usize>;
    fn tag_word_end_boundary(&self, start: usize) -> bool;
}

impl FindExt for &str {
    fn find_case_sensitive(&self, start: usize, pattern: &str) -> Option<Range<usize>> {
        let mut needle_it = pattern.chars();
        if let Some(mut needle_next_ch) = needle_it.next() {
            let mut start: usize = start;
            let mut end: usize = start;
            let mut hey_it = self[start..].chars();
            loop {
                if let Some(hey_ch) = hey_it.next() {
                    let hey_ch_len = hey_ch.len_utf8();
                    let needle_ch = needle_next_ch;
                    if needle_ch == hey_ch {
                        // Found next character of needle:
                        end = end + hey_ch_len;
                        if let Some(ch) = needle_it.next() {
                            needle_next_ch = ch;
                        } else {
                            // Found complete needle:
                            return Some(start..end);
                        }
                    } else {
                        // Restart needle iterator:
                        needle_it = pattern.chars();
                        needle_next_ch = needle_it.next().unwrap();
                        // Restart heystack iterator, but skip first character:
                        hey_it = self[start..].chars();
                        let hey_ch = hey_it.next().unwrap();
                        start = start + hey_ch.len_utf8();
                        end = start;
                    }
                } else {
                    // No more characters in heystack.
                    return None;
                }
            }
        } else {
            // Empty needle matches.
            Some(start..start)
        }
    }

    fn find_case_insensitive(
        &self,
        start: usize,
        upper_case_pattern: &str,
    ) -> Option<Range<usize>> {
        let mut needle_it = upper_case_pattern.chars();
        if let Some(mut needle_next_ch) = needle_it.next() {
            let mut start: usize = start;
            let mut end: usize = start;
            let mut hey_it = self[start..].chars();
            'outer: loop {
                if let Some(hey_ch) = hey_it.next() {
                    let hey_ch_len = hey_ch.len_utf8();
                    end = end + hey_ch_len;
                    let mut hey_ch_upper_it = hey_ch.to_uppercase();
                    while let Some(hey_ch_upper) = hey_ch_upper_it.next() {
                        let needle_ch = needle_next_ch;
                        if needle_ch == hey_ch_upper {
                            // Found next character of needle:
                            if let Some(ch) = needle_it.next() {
                                needle_next_ch = ch;
                            } else {
                                // Found complete needle:
                                return Some(start..end);
                            }
                        } else {
                            // Restart needle iterator:
                            needle_it = upper_case_pattern.chars();
                            needle_next_ch = needle_it.next().unwrap();
                            // Restart heystack iterator, but skip first character:
                            hey_it = self[start..].chars();
                            let hey_ch = hey_it.next().unwrap();
                            start = start + hey_ch.len_utf8();
                            end = start;
                            continue 'outer;
                        }
                    }
                } else {
                    // No more characters in heystack.
                    return None;
                }
            }
        } else {
            // Empty needle matches.
            Some(start..start)
        }
    }

    fn skip_character(&self, start: usize) -> usize {
        let mut it = self[start..].chars();
        let skip = if let Some(ch) = it.next() {
            ch.len_utf8()
        } else {
            0
        };
        start + skip
    }

    fn skip_smart_space(&self, start: usize) -> usize {
        let mut it = self[start..].chars();
        let skip = if let Some(ch) = it.next() {
            let len = ch.len_utf8();
            if ch.is_whitespace() {
                len
            } else if ch == '-' {
                len
            } else if ch == '_' {
                len
            } else {
                0
            }
        } else {
            0
        };
        start + skip
    }

    fn tag_case_sensitive(&self, start: usize, pattern: &str) -> Option<Range<usize>> {
        let mut hey_it = self[start..].chars();
        let mut needle_it = pattern.chars();
        let mut end = start;
        while let Some(needle_ch) = needle_it.next() {
            if let Some(hey_ch) = hey_it.next() {
                if hey_ch == needle_ch {
                    end = end + hey_ch.len_utf8();
                } else {
                    return None;
                }
            } else {
                return None;
            }
        }
        Some(start..end)
    }

    fn tag_case_insensitive(&self, start: usize, upper_case_pattern: &str) -> Option<Range<usize>> {
        let mut hey_it = self[start..].chars();
        let mut needle_it = upper_case_pattern.chars();
        if let Some(mut needle_ch) = needle_it.next() {
            let mut end = start;
            loop {
                if let Some(hey_ch) = hey_it.next() {
                    end = end + hey_ch.len_utf8();
                    let mut hey_ch_upper_it = hey_ch.to_uppercase();
                    while let Some(hey_ch_upper) = hey_ch_upper_it.next() {
                        if hey_ch_upper == needle_ch {
                            // Found next character of needle:
                            if let Some(ch) = needle_it.next() {
                                needle_ch = ch;
                            } else {
                                // Found complete needle:
                                return Some(start..end);
                            }
                        } else {
                            return None;
                        }
                    }
                } else {
                    return None;
                }
            }
        } else {
            Some(start..start)
        }
    }

    fn find_word_start_boundary(&self, start: usize) -> Option<usize> {
        let mut pos = start;
        if pos == self.len() {
            return None;
        }
        let mut previous = 0;
        if pos == 0 {
            let mut it = self.chars();
            if let Some(first) = it.next() {
                if first.is_alphanumeric() {
                    return Some(0);
                } else {
                    pos = first.len_utf8();
                }
            } else {
                return None;
            }
        } else {
            // Find start of previous character:
            previous = pos - 1;
            while !self.is_char_boundary(previous) {
                previous = previous - 1;
            }
        };

        // Here self contains atleast one character.
        let mut it = self[previous..].chars();
        let ch1 = it.next().unwrap();
        let mut ch1 = Features::new(ch1);
        while let Some(ch2) = it.next() {
            let ch2 = Features::new(ch2);
            if !ch1.is_alphabetic && !ch1.is_numeric && (ch2.is_alphabetic || ch2.is_numeric) {
                return Some(pos);
            } else if ch1.is_numeric && ch2.is_alphabetic {
                return Some(pos);
            } else if ch1.is_alphabetic && ch2.is_numeric {
                return Some(pos);
            } else if ch1.is_lower && ch2.is_upper {
                return Some(pos);
            }
            pos = pos + ch2.ch.len_utf8();
            ch1 = ch2;
        }
        None
    }

    fn tag_word_end_boundary(&self, start: usize) -> bool {
        if start == 0 {
            return false;
        }
        let mut previous = start - 1;
        while !self.is_char_boundary(previous) {
            previous = previous - 1;
        }
        let mut it = self[previous..].chars();
        let ch1 = it.next().unwrap();
        if start == self.len() {
            if ch1.is_alphanumeric() {
                return true;
            }
        }
        let ch2 = it.next().unwrap();
        let ch1 = Features::new(ch1);
        let ch2 = Features::new(ch2);
        if (ch1.is_alphabetic || ch1.is_numeric) && !ch2.is_alphabetic && !ch2.is_numeric {
            true
        } else if ch1.is_numeric && ch2.is_alphabetic {
            true
        } else if ch1.is_alphabetic && ch2.is_numeric {
            true
        } else if ch1.is_lower && ch2.is_upper {
            true
        } else {
            false
        }
    }
}

#[derive(Clone, Copy)]
struct Features {
    ch: char,
    is_alphabetic: bool,
    is_lower: bool,
    is_upper: bool,
    is_numeric: bool,
}

impl Features {
    pub fn new(ch: char) -> Features {
        Features {
            ch,
            is_alphabetic: ch.is_alphabetic(),
            is_lower: ch.is_lowercase(),
            is_upper: ch.is_uppercase(),
            is_numeric: ch.is_numeric(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_case_sensitive() {
        assert_eq!("".find_case_sensitive(0, "foo"), None);
        assert_eq!("foo".find_case_sensitive(0, ""), Some(0..0));
        assert_eq!("foo foO foO".find_case_sensitive(0, "fOo"), None);
        assert_eq!("foo foo".find_case_sensitive(0, "foo"), Some(0..3));
        assert_eq!("foo foO foO".find_case_sensitive(0, "foO"), Some(4..7));
        assert_eq!("foo foo foO".find_case_sensitive(0, "foO"), Some(8..11));
    }

    #[test]
    fn test_find_case_sensitive_with_start_offset() {
        assert_eq!("foo".find_case_sensitive(3, ""), Some(3..3));
        assert_eq!("foo foO foO".find_case_sensitive(3, "fOo"), None);
        assert_eq!("foo foo foo".find_case_sensitive(3, "foo"), Some(4..7));
        assert_eq!("foo foO foO".find_case_sensitive(3, "foO"), Some(4..7));
        assert_eq!("foo foo foO".find_case_sensitive(3, "foO"), Some(8..11));
    }

    #[test]
    fn test_find_case_sensitive_multibyte() {
        assert_eq!("ööö".find_case_sensitive(0, ""), Some(0..0));
        assert_eq!("ööö ööÖ ööÖ".find_case_sensitive(0, "öÖö"), None);
        assert_eq!("ööö ööö".find_case_sensitive(0, "ööö"), Some(0..6));
        assert_eq!("ööö ööÖ ööÖ".find_case_sensitive(0, "ööÖ"), Some(7..13));
        assert_eq!("ööö ööö ööÖ".find_case_sensitive(0, "ööÖ"), Some(14..20));
        assert_eq!("äöüÄÖÜß".len(), 14);
    }

    #[test]
    fn test_find_case_sensitive_multibyte_with_start_offset() {
        assert_eq!("ööö".find_case_sensitive(6, ""), Some(6..6));
        assert_eq!("ööö ööÖ ööÖ".find_case_sensitive(6, "öÖö"), None);
        assert_eq!("ööö ööö ööö".find_case_sensitive(6, "ööö"), Some(7..13));
        assert_eq!("ööö ööÖ ööÖ".find_case_sensitive(6, "ööÖ"), Some(7..13));
        assert_eq!("ööö ööö ööÖ".find_case_sensitive(6, "ööÖ"), Some(14..20));
    }

    #[test]
    fn test_find_case_insensitive() {
        assert_eq!("".find_case_insensitive(0, "Foo"), None);
        assert_eq!("foo".find_case_insensitive(0, ""), Some(0..0));
        assert_eq!("fop foP foP".find_case_insensitive(0, "FOO"), None);
        assert_eq!("foO foo".find_case_insensitive(0, "FOO"), Some(0..3));
        assert_eq!("fop foO foO".find_case_insensitive(0, "FOO"), Some(4..7));
        assert_eq!("bar baz Foo".find_case_insensitive(0, "FOO"), Some(8..11));
        assert_eq!("bar baß Foo".find_case_insensitive(0, "FOO"), Some(9..12));
    }

    #[test]
    fn test_find_case_insensitive_with_start_offset() {
        assert_eq!("foo".find_case_insensitive(3, ""), Some(3..3));
        assert_eq!("fop foP foP".find_case_insensitive(3, "FOO"), None);
        assert_eq!("fop FOO foo".find_case_insensitive(3, "FOO"), Some(4..7));
        assert_eq!("fop foO foO".find_case_insensitive(0, "FOO"), Some(4..7));
        assert_eq!("bar baz Foo".find_case_insensitive(0, "FOO"), Some(8..11));
    }

    #[test]
    fn test_find_case_insensitive_multibyte() {
        assert_eq!("ööö".find_case_insensitive(0, ""), Some(0..0));
        assert_eq!("aaa öÖä aöÖ".find_case_insensitive(0, "ÖÖÖ"), None);
        assert_eq!("Ööö Ööö".find_case_insensitive(0, "ÖÖÖ"), Some(0..6));
        assert_eq!("aöö öÖö ööÖ".find_case_insensitive(0, "ÖÖÖ"), Some(6..12));
        assert_eq!("öüö öaö ööÖ".find_case_insensitive(0, "ÖÖÖ"), Some(13..19));
        assert_eq!("bar baz Fuß".find_case_insensitive(0, "FUS"), Some(8..12));
        assert_eq!("bar baz Fuß".find_case_insensitive(0, "FUSS"), Some(8..12));
    }

    #[test]
    fn test_find_case_insensitive_multibyte_with_start_offset() {
        assert_eq!("ööö".find_case_insensitive(6, ""), Some(6..6));
        assert_eq!("aaa öÖä aöÖ".find_case_insensitive(6, "ÖÖÖ"), None);
        assert_eq!("Ööö Ööö".find_case_insensitive(6, "ÖÖÖ"), Some(7..13));
        assert_eq!("aöö öÖö ööÖ".find_case_insensitive(6, "ÖÖÖ"), Some(6..12));
        assert_eq!("öüö öaö ööÖ".find_case_insensitive(6, "ÖÖÖ"), Some(13..19));
    }

    #[test]
    fn test_skip_smart_space() {
        assert_eq!("foo bar".skip_smart_space(2), 2);
        assert_eq!("foo bar".skip_smart_space(3), 4);
        assert_eq!("foo-bar".skip_smart_space(3), 4);
        assert_eq!("foo_bar".skip_smart_space(3), 4);
        assert_eq!("foo bar".skip_smart_space(4), 4);
    }

    #[test]
    fn test_skip_character() {
        assert_eq!("foo bar".skip_character(2), 3);
        assert_eq!("1ä".skip_character(1), 3); // 0xC3, 0xA4 (ä)
        assert_eq!("1ä".skip_character(1), 2); // 0x61 (a), 0xCC, 0x88 (Trema for previous letter)
    }

    #[test]
    fn test_tag_case_sensitive() {
        assert_eq!("".tag_case_sensitive(0, "foo"), None);
        assert_eq!("foo bar baz".tag_case_sensitive(0, ""), Some(0..0));
        assert_eq!("foo bar baz".tag_case_sensitive(5, ""), Some(5..5));
        assert_eq!("foo bar baz".tag_case_sensitive(11, ""), Some(11..11));
        assert_eq!("foo bar baz".tag_case_sensitive(0, "foo"), Some(0..3));
        assert_eq!("foo bar baz".tag_case_sensitive(4, "bar"), Some(4..7));
        assert_eq!("foo bar baz".tag_case_sensitive(3, "bar"), None);
        assert_eq!("foo bar baz".tag_case_sensitive(8, "baz"), Some(8..11));
        assert_eq!("foo bar baz".tag_case_sensitive(8, "bazz"), None);
    }

    #[test]
    fn test_tag_case_sensitive_multi_byte() {
        assert_eq!("föo bar baz".tag_case_sensitive(0, "föo"), Some(0..4));
        assert_eq!("föo bär baz".tag_case_sensitive(5, "bär"), Some(5..9));
        assert_eq!("föo bär baz".tag_case_sensitive(4, "bär"), None);
        assert_eq!("foo bär baü".tag_case_sensitive(9, "baü"), Some(9..13));
        assert_eq!("foo bär baü".tag_case_sensitive(8, "baü"), None);
    }

    #[test]
    fn test_tag_case_insensitive() {
        assert_eq!("".tag_case_insensitive(0, "FOO"), None);
        assert_eq!("fOo bar baz".tag_case_insensitive(0, ""), Some(0..0));
        assert_eq!("foO bar baz".tag_case_insensitive(5, ""), Some(5..5));
        assert_eq!("foo bar bAz".tag_case_insensitive(11, ""), Some(11..11));
        assert_eq!("Foo bar baz".tag_case_insensitive(0, "FOO"), Some(0..3));
        assert_eq!("foo bAr baz".tag_case_insensitive(4, "BAR"), Some(4..7));
        assert_eq!("foo bar baZ".tag_case_insensitive(8, "BAZ"), Some(8..11));
        assert_eq!("foo bar baZ".tag_case_insensitive(8, "BAZZ"), None);
    }

    #[test]
    fn test_tag_case_insensitive_multi_byte() {
        assert_eq!("fÖo bar baz".tag_case_insensitive(0, "FÖO"), Some(0..4));
        assert_eq!("fÖo bÄr baz".tag_case_insensitive(5, "BÄR"), Some(5..9));
        assert_eq!("föo bÄr baz".tag_case_insensitive(4, "BÄR"), None);
        assert_eq!("foo bär baÜ".tag_case_insensitive(9, "BAÜ"), Some(9..13));
        assert_eq!("foo bär baÜ".tag_case_insensitive(8, "BAÜ"), None);
        assert_eq!("foo bär fuß".tag_case_insensitive(9, "FUS"), Some(9..13));
        assert_eq!("foo bär fuß".tag_case_insensitive(9, "FUSS"), Some(9..13));
    }

    #[test]
    fn test_find_word_start_boundary() {
        assert_eq!("".find_word_start_boundary(0), None);
        assert_eq!("foo".find_word_start_boundary(0), Some(0));
        assert_eq!("foo".find_word_start_boundary(3), None);
        assert_eq!(" foo".find_word_start_boundary(0), Some(1));
        assert_eq!("  foo".find_word_start_boundary(0), Some(2));
        assert_eq!("  Foo".find_word_start_boundary(0), Some(2));
        assert_eq!("  123".find_word_start_boundary(0), Some(2));
        assert_eq!("a foo".find_word_start_boundary(1), Some(2));
        assert_eq!("foo bar".find_word_start_boundary(1), Some(4));
        assert_eq!("foo bar".find_word_start_boundary(1), Some(4));
        assert_eq!("Foobar".find_word_start_boundary(1), None);
        assert_eq!("FooBar".find_word_start_boundary(1), Some(3));
        assert_eq!("Foo123".find_word_start_boundary(1), Some(3));
        assert_eq!("123Foo".find_word_start_boundary(1), Some(3));
    }

    #[test]
    fn test_tag_word_end_boundary() {
        assert_eq!("".tag_word_end_boundary(0), false);
        assert_eq!("foo".tag_word_end_boundary(0), false);
        assert_eq!("foo".tag_word_end_boundary(1), false);
        assert_eq!("foo".tag_word_end_boundary(2), false);
        assert_eq!("foo".tag_word_end_boundary(3), true);
        assert_eq!("foo ".tag_word_end_boundary(3), true);
        assert_eq!("123".tag_word_end_boundary(0), false);
        assert_eq!("123".tag_word_end_boundary(1), false);
        assert_eq!("123".tag_word_end_boundary(2), false);
        assert_eq!("123".tag_word_end_boundary(3), true);
        assert_eq!("123 ".tag_word_end_boundary(3), true);
        assert_eq!("foo123".tag_word_end_boundary(3), true);
        assert_eq!("123foo".tag_word_end_boundary(3), true);
        assert_eq!("FooBar".tag_word_end_boundary(3), true);
        assert_eq!("foobar".tag_word_end_boundary(3), false);
        assert_eq!("123456".tag_word_end_boundary(3), false);
        assert_eq!("------".tag_word_end_boundary(3), false);
    }
}
