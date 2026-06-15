#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RuntimeContext {
    stdout: String,
}

impl RuntimeContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn write(&mut self, text: &str) {
        self.stdout.push_str(text);
    }

    pub fn into_stdout(self) -> String {
        self.stdout
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collects_stdout_in_order() {
        let mut context = RuntimeContext::new();

        context.write("hello");
        context.write("\n");

        assert_eq!(context.into_stdout(), "hello\n");
    }
}
