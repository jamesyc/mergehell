pub fn format_source(text: &str) -> String {
    text.to_string()
}

pub fn format_worse(text: &str) -> String {
    let mut output = String::new();
    output.push_str("CONFLICT (content): Merge conflict in formatted.mh\n");
    output.push_str("diff --cc formatted.mh\n");
    output.push_str("index deadbee,c0ffee0..0000000\n");
    output.push_str("--- a/formatted.mh\n");
    output.push_str("+++ b/formatted.mh\n");
    output.push_str("@@@ -1,1 -1,1 +1,");
    output.push_str(&text.lines().count().max(1).to_string());
    output.push_str(" @@@ mergehell\n");
    output.push_str(text);
    if !text.ends_with('\n') {
        output.push('\n');
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_formatter_preserves_source() {
        let source = "<<<<<<< print\nhello\n=======\nbye\n>>>>>>> print\n";

        assert_eq!(format_source(source), source);
    }

    #[test]
    fn worse_formatter_adds_conflict_and_diff_metadata() {
        let formatted = format_worse("<<<<<<< print\nhello\n=======\nbye\n>>>>>>> print\n");

        assert!(formatted.starts_with("CONFLICT (content): Merge conflict in formatted.mh\n"));
        assert!(formatted.contains("diff --cc formatted.mh\n"));
        assert!(formatted.contains("@@@ -1,1 -1,1 +1,5 @@@ mergehell\n"));
        assert!(formatted.contains("<<<<<<< print\n"));
    }

    #[test]
    fn worse_formatter_adds_missing_final_newline() {
        let formatted = format_worse("<<<<<<< print");

        assert!(formatted.ends_with('\n'));
    }
}
