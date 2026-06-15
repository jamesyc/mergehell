pub fn format_source(text: &str) -> String {
    text.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_formatter_preserves_source() {
        let source = "<<<<<<< print\nhello\n=======\nbye\n>>>>>>> print\n";

        assert_eq!(format_source(source), source);
    }
}
