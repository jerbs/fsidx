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
                let needle_ch = needle_next_ch;
                if let Some(hey_ch) = hey_it.next() {
                    let hey_ch_len = hey_ch.len_utf8();
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

    fn find_case_insensitive(&self, start: usize, pattern: &str) -> Option<Range<usize>> {
        todo!()
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
        assert_eq!("".find_case_sensitive(0, "foo"), None);
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

}
