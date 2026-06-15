pub type FileId = usize;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceName(pub String);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Span {
    pub file_id: FileId,
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceFile {
    pub id: FileId,
    pub name: SourceName,
    pub text: String,
    pub line_starts: Vec<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceLine<'a> {
    pub number: usize,
    pub text: &'a str,
    pub span: Span,
}

pub fn decode_source_bytes(name: &str, bytes: Vec<u8>) -> String {
    match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(_) => binary_conflict_source(name),
    }
}

pub fn binary_conflict_source(name: &str) -> String {
    format!(
        "CONFLICT (binary): Merge conflict in {name}\n<<<<<<< binary\n<opaque bytes>\n=======\n<opaque bytes>\n>>>>>>> binary\n"
    )
}

impl SourceFile {
    pub fn new(name: impl Into<String>, text: impl Into<String>) -> Self {
        let text = text.into();
        let mut line_starts = vec![0];
        for (index, byte) in text.bytes().enumerate() {
            if byte == b'\n' && index + 1 < text.len() {
                line_starts.push(index + 1);
            }
        }

        Self {
            id: 0,
            name: SourceName(name.into()),
            text,
            line_starts,
        }
    }

    pub fn lines(&self) -> SourceLines<'_> {
        SourceLines {
            source: self,
            next_line: 0,
        }
    }

    pub fn span(&self) -> Span {
        Span {
            file_id: self.id,
            start: 0,
            end: self.text.len(),
        }
    }

    pub fn line_col(&self, offset: usize) -> (usize, usize) {
        let clamped = offset.min(self.text.len());
        let index = match self.line_starts.binary_search(&clamped) {
            Ok(index) => index,
            Err(index) => index.saturating_sub(1),
        };
        let line_start = self.line_starts[index];
        (index + 1, clamped - line_start + 1)
    }
}

pub struct SourceLines<'a> {
    source: &'a SourceFile,
    next_line: usize,
}

impl<'a> Iterator for SourceLines<'a> {
    type Item = SourceLine<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.source.text.is_empty() || self.next_line >= self.source.line_starts.len() {
            return None;
        }

        let line_index = self.next_line;
        let start = self.source.line_starts[line_index];
        let end = self
            .source
            .line_starts
            .get(line_index + 1)
            .copied()
            .unwrap_or(self.source.text.len());
        self.next_line += 1;

        let mut text_end = end;
        if text_end > start && self.source.text.as_bytes()[text_end - 1] == b'\n' {
            text_end -= 1;
        }
        if text_end > start && self.source.text.as_bytes()[text_end - 1] == b'\r' {
            text_end -= 1;
        }

        Some(SourceLine {
            number: line_index + 1,
            text: &self.source.text[start..text_end],
            span: Span {
                file_id: self.source.id,
                start,
                end,
            },
        })
    }
}

impl Span {
    pub fn join(self, other: Span) -> Span {
        Span {
            file_id: self.file_id,
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_line_starts_for_final_newline() {
        let source = SourceFile::new("test.mh", "one\ntwo\n");

        assert_eq!(source.line_starts, vec![0, 4]);
        assert_eq!(
            source.lines().map(|line| line.text).collect::<Vec<_>>(),
            vec!["one", "two"]
        );
    }

    #[test]
    fn computes_line_starts_without_final_newline() {
        let source = SourceFile::new("test.mh", "one\ntwo");

        assert_eq!(source.line_starts, vec![0, 4]);
        assert_eq!(
            source.lines().map(|line| line.text).collect::<Vec<_>>(),
            vec!["one", "two"]
        );
    }

    #[test]
    fn empty_source_has_no_lines() {
        let source = SourceFile::new("empty.mh", "");

        assert_eq!(source.lines().count(), 0);
    }

    #[test]
    fn maps_offsets_to_line_and_column() {
        let source = SourceFile::new("test.mh", "one\ntwo");

        assert_eq!(source.line_col(0), (1, 1));
        assert_eq!(source.line_col(4), (2, 1));
        assert_eq!(source.line_col(99), (2, 4));
    }

    #[test]
    fn joins_spans() {
        let left = Span {
            file_id: 0,
            start: 5,
            end: 10,
        };
        let right = Span {
            file_id: 0,
            start: 1,
            end: 6,
        };

        assert_eq!(
            left.join(right),
            Span {
                file_id: 0,
                start: 1,
                end: 10
            }
        );
    }

    #[test]
    fn decodes_utf8_source_bytes() {
        assert_eq!(
            decode_source_bytes("test.mh", b"hello\n".to_vec()),
            "hello\n"
        );
    }

    #[test]
    fn invalid_utf8_becomes_binary_conflict_source() {
        let source = decode_source_bytes("image.bin", vec![0xff, 0xfe]);

        assert!(source.contains("CONFLICT (binary): Merge conflict in image.bin"));
        assert!(source.contains("<<<<<<< binary"));
        assert!(source.contains(">>>>>>> binary"));
    }
}
