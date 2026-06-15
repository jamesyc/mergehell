pub fn append_line(output: &mut String, line: &str) {
    output.push_str(line);
    output.push('\n');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_line_with_newline() {
        let mut output = String::new();

        append_line(&mut output, "hello");

        assert_eq!(output, "hello\n");
    }

    #[test]
    fn empty_line_still_outputs_newline() {
        let mut output = String::new();

        append_line(&mut output, "");

        assert_eq!(output, "\n");
    }
}
