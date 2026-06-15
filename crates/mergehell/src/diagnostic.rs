use crate::source::Span;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DiagnosticKind {
    Syntax,
    Runtime,
    Type,
    Binary,
    Warning,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Severity {
    Warning,
    Error,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub kind: DiagnosticKind,
    pub severity: Severity,
    pub message: String,
    pub span: Option<Span>,
    pub expected: Option<String>,
    pub actual: Option<String>,
    pub hints: Vec<String>,
}

impl Diagnostic {
    pub fn new(
        kind: DiagnosticKind,
        severity: Severity,
        message: impl Into<String>,
        span: Option<Span>,
    ) -> Self {
        Self {
            kind,
            severity,
            message: message.into(),
            span,
            expected: None,
            actual: None,
            hints: Vec::new(),
        }
    }

    pub fn syntax_error(message: impl Into<String>, span: Option<Span>) -> Self {
        Self::new(DiagnosticKind::Syntax, Severity::Error, message, span)
    }

    pub fn syntax_warning(message: impl Into<String>, span: Option<Span>) -> Self {
        Self::new(DiagnosticKind::Warning, Severity::Warning, message, span)
    }

    pub fn runtime_error(message: impl Into<String>, span: Option<Span>) -> Self {
        Self::new(DiagnosticKind::Runtime, Severity::Error, message, span)
    }

    pub fn with_expected_actual(
        mut self,
        expected: impl Into<String>,
        actual: impl Into<String>,
    ) -> Self {
        self.expected = Some(expected.into());
        self.actual = Some(actual.into());
        self
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hints.push(hint.into());
        self
    }

    pub fn render_mergehell(&self) -> String {
        if self.expected.is_none() && self.actual.is_none() {
            let mut rendered = self.message.clone();
            if !rendered.ends_with('\n') {
                rendered.push('\n');
            }
            for hint in &self.hints {
                rendered.push_str(hint);
                if !hint.ends_with('\n') {
                    rendered.push('\n');
                }
            }
            return rendered;
        }

        let mut rendered = String::new();
        rendered.push_str("CONFLICT (");
        rendered.push_str(self.kind.label());
        rendered.push_str("): ");
        rendered.push_str(&self.message);
        rendered.push('\n');
        rendered.push_str("<<<<<<< expected\n");
        if let Some(expected) = &self.expected {
            rendered.push_str(expected);
            if !expected.ends_with('\n') {
                rendered.push('\n');
            }
        }
        rendered.push_str("=======\n");
        if let Some(actual) = &self.actual {
            rendered.push_str(actual);
            if !actual.ends_with('\n') {
                rendered.push('\n');
            }
        }
        rendered.push_str(">>>>>>> ");
        rendered.push_str(self.kind.label());
        rendered.push('\n');
        for hint in &self.hints {
            rendered.push_str(hint);
            if !hint.ends_with('\n') {
                rendered.push('\n');
            }
        }
        rendered
    }
}

impl DiagnosticKind {
    pub fn label(&self) -> &'static str {
        match self {
            DiagnosticKind::Syntax => "syntax",
            DiagnosticKind::Runtime => "runtime",
            DiagnosticKind::Type => "type",
            DiagnosticKind::Binary => "binary",
            DiagnosticKind::Warning => "warning",
        }
    }
}

pub fn render_diagnostics(diagnostics: &[Diagnostic]) -> String {
    diagnostics
        .iter()
        .map(Diagnostic::render_mergehell)
        .collect::<Vec<_>>()
        .join("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_plain_diagnostic_with_hint() {
        let diagnostic = Diagnostic::runtime_error("fatal: no conflict markers found", None)
            .with_hint("hint: this appears to be valid software");

        assert_eq!(
            diagnostic.render_mergehell(),
            "fatal: no conflict markers found\nhint: this appears to be valid software\n"
        );
    }

    #[test]
    fn renders_structured_diagnostic_as_mergehell_source() {
        let diagnostic = Diagnostic::syntax_error("Merge conflict in parser", None)
            .with_expected_actual(">>>>>>>", "end of file")
            .with_hint("hint: close the conflict");

        assert_eq!(
            diagnostic.render_mergehell(),
            "CONFLICT (syntax): Merge conflict in parser\n<<<<<<< expected\n>>>>>>>\n=======\nend of file\n>>>>>>> syntax\nhint: close the conflict\n"
        );
    }

    #[test]
    fn render_diagnostics_concatenates_all_entries() {
        let diagnostics = vec![
            Diagnostic::runtime_error("first", None),
            Diagnostic::runtime_error("second", None),
        ];

        assert_eq!(render_diagnostics(&diagnostics), "first\nsecond\n");
    }
}
