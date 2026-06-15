use std::collections::HashMap;

use crate::resolve::rng::SeededRng;
use crate::syntax::ast::Node;

use super::value::Value;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Function {
    pub params: Vec<String>,
    pub body: Vec<Node>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeContext {
    stdout: String,
    scopes: Vec<HashMap<String, Value>>,
    functions: HashMap<String, Function>,
    rng: SeededRng,
}

impl RuntimeContext {
    pub fn new(seed: u64) -> Self {
        Self {
            stdout: String::new(),
            scopes: vec![HashMap::new()],
            functions: HashMap::new(),
            rng: SeededRng::new(seed),
        }
    }

    pub fn write(&mut self, text: &str) {
        self.stdout.push_str(text);
    }

    pub fn capture_output<F>(&mut self, f: F) -> Result<String, Vec<crate::diagnostic::Diagnostic>>
    where
        F: FnOnce(&mut Self) -> Result<(), Vec<crate::diagnostic::Diagnostic>>,
    {
        let start = self.stdout.len();
        f(self)?;
        Ok(self.stdout.split_off(start))
    }

    pub fn set_var(&mut self, name: impl Into<String>, value: Value) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.into(), value);
        }
    }

    pub fn get_var(&self, name: &str) -> Option<&Value> {
        self.scopes.iter().rev().find_map(|scope| scope.get(name))
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    pub fn define_function(&mut self, name: impl Into<String>, function: Function) {
        self.functions.insert(name.into(), function);
    }

    pub fn get_function(&self, name: &str) -> Option<&Function> {
        self.functions.get(name)
    }

    pub fn choose_index(&mut self, len: usize) -> Option<usize> {
        self.rng.choose_index(len)
    }

    pub fn into_stdout(self) -> String {
        self.stdout
    }
}

impl Default for RuntimeContext {
    fn default() -> Self {
        Self::new(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collects_stdout_in_order() {
        let mut context = RuntimeContext::new(0);

        context.write("hello");
        context.write("\n");

        assert_eq!(context.into_stdout(), "hello\n");
    }

    #[test]
    fn captures_output_without_losing_side_effects() {
        let mut context = RuntimeContext::new(0);

        context.write("before\n");
        let captured = context
            .capture_output(|context| {
                context.write("captured\n");
                context.set_var("name", Value::String("James".to_string()));
                Ok(())
            })
            .unwrap();

        assert_eq!(captured, "captured\n");
        assert_eq!(
            context.get_var("name"),
            Some(&Value::String("James".to_string()))
        );
        assert_eq!(context.into_stdout(), "before\n");
    }

    #[test]
    fn variables_resolve_from_inner_to_outer_scope() {
        let mut context = RuntimeContext::new(0);

        context.set_var("name", Value::String("outer".to_string()));
        context.push_scope();
        context.set_var("name", Value::String("inner".to_string()));

        assert_eq!(
            context.get_var("name"),
            Some(&Value::String("inner".to_string()))
        );
        context.pop_scope();
        assert_eq!(
            context.get_var("name"),
            Some(&Value::String("outer".to_string()))
        );
    }

    #[test]
    fn root_scope_is_not_popped() {
        let mut context = RuntimeContext::new(0);

        context.pop_scope();
        context.set_var("name", Value::String("root".to_string()));

        assert_eq!(
            context.get_var("name"),
            Some(&Value::String("root".to_string()))
        );
    }

    #[test]
    fn stores_functions() {
        let mut context = RuntimeContext::new(0);
        let function = Function {
            params: vec!["name".to_string()],
            body: Vec::new(),
        };

        context.define_function("greet", function.clone());

        assert_eq!(context.get_function("greet"), Some(&function));
    }

    #[test]
    fn seeded_choice_is_deterministic() {
        let mut left = RuntimeContext::new(7);
        let mut right = RuntimeContext::new(7);

        assert_eq!(left.choose_index(3), right.choose_index(3));
    }
}
