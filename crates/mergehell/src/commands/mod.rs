pub mod binding;
pub mod control_flow;
pub mod functions;
pub mod import;
pub mod print;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommandDispatch {
    Print,
    Transparent,
}

pub fn dispatch_for(command_name: &str) -> CommandDispatch {
    match command_name {
        "print" => CommandDispatch::Print,
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
    fn unknown_command_is_transparent() {
        assert_eq!(dispatch_for("HEAD"), CommandDispatch::Transparent);
        assert_eq!(dispatch_for(""), CommandDispatch::Transparent);
    }
}
