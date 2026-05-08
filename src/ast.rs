use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileSpan<'a> {
    pub(crate) source: &'a str,
    pub(crate) chars: Range<usize>,
}

impl<'a> FileSpan<'a> {
    pub fn new(source: &'a str, chars: Range<usize>) -> FileSpan<'a> {
        FileSpan {
            source,
            chars,
        }
    }
}
