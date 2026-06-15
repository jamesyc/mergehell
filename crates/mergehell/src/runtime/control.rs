#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EvalOutcome {
    Unit,
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
}
