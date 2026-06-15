use std::collections::HashMap;
use std::path::{Path, PathBuf};

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
    current_dir: Option<PathBuf>,
    import_stack: Vec<PathBuf>,
}

impl RuntimeContext {
    pub fn new(seed: u64) -> Self {
        Self {
            stdout: String::new(),
            scopes: vec![HashMap::new()],
            functions: HashMap::new(),
            rng: SeededRng::new(seed),
            current_dir: None,
            import_stack: Vec::new(),
        }
    }

    pub fn with_source_name(mut self, source_name: &str) -> Self {
        let path = Path::new(source_name);
        self.current_dir = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .map(Path::to_path_buf);
        self
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

    pub fn resolve_import_path(&self, raw_path: &str) -> PathBuf {
        let path = PathBuf::from(raw_path);
        if path.is_absolute() {
            path
        } else if let Some(current_dir) = &self.current_dir {
            current_dir.join(path)
        } else {
            path
        }
    }

    pub fn replace_current_dir(&mut self, current_dir: Option<PathBuf>) -> Option<PathBuf> {
        std::mem::replace(&mut self.current_dir, current_dir)
    }

    pub fn enter_import(&mut self, path: PathBuf) -> bool {
        if self.import_stack.contains(&path) {
            false
        } else {
            self.import_stack.push(path);
            true
        }
    }

    pub fn leave_import(&mut self) {
        self.import_stack.pop();
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

    #[test]
    fn resolves_imports_relative_to_source_file() {
        let context = RuntimeContext::new(0).with_source_name("/tmp/programs/main.mh");

        assert_eq!(
            context.resolve_import_path("lib.mh"),
            PathBuf::from("/tmp/programs/lib.mh")
        );
        assert_eq!(
            context.resolve_import_path("/absolute/lib.mh"),
            PathBuf::from("/absolute/lib.mh")
        );
    }

    #[test]
    fn detects_import_cycles_on_active_stack() {
        let mut context = RuntimeContext::new(0);
        let path = PathBuf::from("module.mh");

        assert!(context.enter_import(path.clone()));
        assert!(!context.enter_import(path));
        context.leave_import();
        assert!(context.enter_import(PathBuf::from("module.mh")));
    }
}
