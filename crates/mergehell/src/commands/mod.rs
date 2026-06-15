pub mod binding;
pub mod control_flow;
pub mod functions;
pub mod import;
pub mod print;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommandDispatch {
    Print,
    Let,
    If,
    Repeat,
    Function,
    Call,
    Return,
    Try,
    Throw,
    Import,
    Resolve,
    Transparent,
}

pub fn dispatch_for(command_name: &str) -> CommandDispatch {
    match command_name {
        "print" => CommandDispatch::Print,
        "let" => CommandDispatch::Let,
        "if" => CommandDispatch::If,
        "repeat" => CommandDispatch::Repeat,
        "function" => CommandDispatch::Function,
        "call" => CommandDispatch::Call,
        "return" => CommandDispatch::Return,
        "try" => CommandDispatch::Try,
        "throw" => CommandDispatch::Throw,
        "import" => CommandDispatch::Import,
        "resolve" => CommandDispatch::Resolve,
        _ => CommandDispatch::Transparent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_dispatches_to_print_command() {
        assert_eq!(dispatch_for("print"), CommandDispatch::Print);
    }

    #[test]
    fn level_one_commands_dispatch_to_command_handlers() {
        assert_eq!(dispatch_for("let"), CommandDispatch::Let);
        assert_eq!(dispatch_for("if"), CommandDispatch::If);
        assert_eq!(dispatch_for("repeat"), CommandDispatch::Repeat);
        assert_eq!(dispatch_for("function"), CommandDispatch::Function);
        assert_eq!(dispatch_for("call"), CommandDispatch::Call);
        assert_eq!(dispatch_for("return"), CommandDispatch::Return);
        assert_eq!(dispatch_for("try"), CommandDispatch::Try);
        assert_eq!(dispatch_for("throw"), CommandDispatch::Throw);
        assert_eq!(dispatch_for("import"), CommandDispatch::Import);
        assert_eq!(dispatch_for("resolve"), CommandDispatch::Resolve);
    }

    #[test]
    fn unknown_command_is_transparent() {
        assert_eq!(dispatch_for("HEAD"), CommandDispatch::Transparent);
        assert_eq!(dispatch_for(""), CommandDispatch::Transparent);
    }
}
