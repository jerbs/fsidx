use std::ops::Range;

pub trait FindExt {
    fn find_case_sensitive(&self, start: usize, pattern: &str) -> Option<Range<usize>>;
    fn find_case_insensitive(&self, start: usize, pattern: &str) -> Option<Range<usize>>;
    fn skip_smart_space(&self, start: usize) -> usize;
    fn tag_case_sensitive(&self, start: usize, pattern: &str) -> Option<Range<usize>>;
    fn tag_case_insensitive(&self, start: usize, pattern: &str) -> Option<Range<usize>>;
    fn find_word_boundary(&self, start: usize) -> Option<usize>;
    fn tag_word_boundary(&self, start: usize) -> bool;
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

    fn find_case_insensitive(&self, start: usize, lower_case_pattern: &str) -> Option<Range<usize>> {
        let mut needle_it = lower_case_pattern.chars();
        if let Some(mut needle_next_ch) = needle_it.next() {
            let mut start: usize = start;
            let mut end: usize = start;
            let mut hey_it = self[start..].chars();
            'outer: loop {
                if let Some(hey_ch) = hey_it.next() {
                    let hey_ch_len = hey_ch.len_utf8();
                    let mut hey_ch_lower_it = hey_ch.to_lowercase();
                    while let Some(hey_ch_lower) = hey_ch_lower_it.next() {
                        let needle_ch = needle_next_ch;
                        if needle_ch == hey_ch_lower {
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
                            needle_it = lower_case_pattern.chars();
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

    fn skip_smart_space(&self, start: usize) -> usize {
        todo!()
    }

    fn tag_case_sensitive(&self, start: usize, pattern: &str) -> Option<Range<usize>> {
        todo!()
    }

    fn tag_case_insensitive(&self, start: usize, pattern: &str) -> Option<Range<usize>> {
        todo!()
    }

    fn find_word_boundary(&self, start: usize) -> Option<usize> {
        todo!()
    }

    fn tag_word_boundary(&self, start: usize) -> bool {
        todo!()
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
        assert_eq!("".find_case_insensitive(0, "foo"), None);
        assert_eq!("foo".find_case_insensitive(0, ""), Some(0..0));
        assert_eq!("fop foP foP".find_case_insensitive(0, "foo"), None);
        assert_eq!("foO foo".find_case_insensitive(0, "foo"), Some(0..3));
        assert_eq!("fop foO foO".find_case_insensitive(0, "foo"), Some(4..7));
        assert_eq!("bar baz Foo".find_case_insensitive(0, "foo"), Some(8..11));
    }

    #[test]
    fn test_find_case_insensitive_with_start_offset() {
        assert_eq!("foo".find_case_insensitive(3, ""), Some(3..3));
        assert_eq!("fop foP foP".find_case_insensitive(3, "foo"), None);
        assert_eq!("fop FOO foo".find_case_insensitive(3, "foo"), Some(4..7));
        assert_eq!("fop foO foO".find_case_insensitive(0, "foo"), Some(4..7));
        assert_eq!("bar baz Foo".find_case_insensitive(0, "foo"), Some(8..11));
    }


    #[test]
    fn test_find_case_insensitive_multibyte() {
        assert_eq!("ööö".find_case_insensitive(0, ""), Some(0..0));
        assert_eq!("aaa öÖä aöÖ".find_case_insensitive(0, "ööö"), None);
        assert_eq!("Ööö Ööö".find_case_insensitive(0, "ööö"), Some(0..6));
        assert_eq!("aöö öÖö ööÖ".find_case_insensitive(0, "ööö"), Some(6..12));
        assert_eq!("öüö öaö ööÖ".find_case_insensitive(0, "ööö"), Some(13..19));
    }

    #[test]
    fn test_find_case_insensitive_multibyte_with_start_offset() {
        assert_eq!("ööö".find_case_insensitive(6, ""), Some(6..6));
        assert_eq!("aaa öÖä aöÖ".find_case_insensitive(6, "ööö"), None);
        assert_eq!("Ööö Ööö".find_case_insensitive(6, "ööö"), Some(7..13));
        assert_eq!("aöö öÖö ööÖ".find_case_insensitive(6, "ööö"), Some(6..12));
        assert_eq!("öüö öaö ööÖ".find_case_insensitive(6, "ööö"), Some(13..19));
    }
}
