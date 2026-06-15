use crate::git::status::GitStrategyError;
use crate::resolve::strategy::Strategy;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlameLane {
    Ours,
    Base,
    Theirs,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlameRecord {
    pub lane: BlameLane,
    pub author: String,
    pub author_time: u64,
}

pub trait BlameProvider {
    fn records(&self) -> Result<Vec<BlameRecord>, GitStrategyError>;
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RealBlameProvider;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FakeBlameProvider {
    records: Result<Vec<BlameRecord>, GitStrategyError>,
}

impl FakeBlameProvider {
    pub fn new(records: Vec<BlameRecord>) -> Self {
        Self {
            records: Ok(records),
        }
    }

    pub fn failing(message: impl Into<String>) -> Self {
        Self {
            records: Err(GitStrategyError {
                message: message.into(),
                hint: None,
            }),
        }
    }
}

impl BlameProvider for FakeBlameProvider {
    fn records(&self) -> Result<Vec<BlameRecord>, GitStrategyError> {
        self.records.clone()
    }
}

impl BlameProvider for RealBlameProvider {
    fn records(&self) -> Result<Vec<BlameRecord>, GitStrategyError> {
        Err(GitStrategyError {
            message: "fatal: blame strategy requires blame metadata".to_string(),
            hint: Some(
                "hint: use a fake BlameProvider in tests or pass an explicit strategy".to_string(),
            ),
        })
    }
}

pub fn strategy_from_blame(provider: &dyn BlameProvider) -> Result<Strategy, GitStrategyError> {
    let records = provider.records()?;
    let selected = records
        .iter()
        .max_by_key(|record| record.author_time)
        .ok_or_else(|| GitStrategyError {
            message: "fatal: no blame records available".to_string(),
            hint: Some("hint: use --ours, --theirs, or provide blame metadata".to_string()),
        })?;

    Ok(match selected.lane {
        BlameLane::Ours => Strategy::Ours,
        BlameLane::Base => Strategy::Base,
        BlameLane::Theirs => Strategy::Theirs,
    })
}

pub fn strategy_from_real_blame() -> Result<Strategy, GitStrategyError> {
    strategy_from_blame(&RealBlameProvider)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(lane: BlameLane, author: &str, author_time: u64) -> BlameRecord {
        BlameRecord {
            lane,
            author: author.to_string(),
            author_time,
        }
    }

    #[test]
    fn newest_ours_record_selects_ours() {
        let provider = FakeBlameProvider::new(vec![
            record(BlameLane::Theirs, "remote", 10),
            record(BlameLane::Ours, "local", 20),
        ]);

        assert_eq!(strategy_from_blame(&provider), Ok(Strategy::Ours));
    }

    #[test]
    fn newest_theirs_record_selects_theirs() {
        let provider = FakeBlameProvider::new(vec![
            record(BlameLane::Ours, "local", 10),
            record(BlameLane::Theirs, "remote", 20),
        ]);

        assert_eq!(strategy_from_blame(&provider), Ok(Strategy::Theirs));
    }

    #[test]
    fn newest_base_record_selects_base() {
        let provider = FakeBlameProvider::new(vec![
            record(BlameLane::Ours, "local", 10),
            record(BlameLane::Base, "ancestor", 20),
        ]);

        assert_eq!(strategy_from_blame(&provider), Ok(Strategy::Base));
    }

    #[test]
    fn empty_blame_records_error() {
        let provider = FakeBlameProvider::new(Vec::new());

        assert_eq!(
            strategy_from_blame(&provider).unwrap_err().message,
            "fatal: no blame records available"
        );
    }

    #[test]
    fn provider_failure_is_returned() {
        let provider = FakeBlameProvider::failing("fatal: blame failed");

        assert_eq!(
            strategy_from_blame(&provider).unwrap_err().message,
            "fatal: blame failed"
        );
    }
}
