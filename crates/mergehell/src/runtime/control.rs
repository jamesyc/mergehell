use super::value::Value;

#[derive(Clone, Debug, PartialEq)]
pub enum EvalOutcome {
    Unit,
    Return(Value),
}

impl EvalOutcome {
    pub fn is_unit(&self) -> bool {
        matches!(self, EvalOutcome::Unit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_outcome_reports_unit() {
        assert!(EvalOutcome::Unit.is_unit());
    }

    #[test]
    fn return_outcome_is_not_unit() {
        assert!(!EvalOutcome::Return(Value::Null).is_unit());
    }
}
