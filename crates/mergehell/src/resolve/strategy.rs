use std::str::FromStr;

use crate::syntax::ast::{ConflictNode, Node};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Strategy {
    Ours,
    Theirs,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LaneName {
    Ours,
    Theirs,
}

#[derive(Debug, Eq, PartialEq)]
pub struct SelectedLane<'a> {
    pub name: LaneName,
    pub nodes: &'a [Node],
}

pub trait Resolver {
    fn select<'a>(&self, conflict: &'a ConflictNode) -> SelectedLane<'a>;
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OursResolver;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TheirsResolver;

impl Strategy {
    pub fn resolver(self) -> Box<dyn Resolver> {
        match self {
            Strategy::Ours => Box::new(OursResolver),
            Strategy::Theirs => Box::new(TheirsResolver),
        }
    }

    pub fn flag(self) -> &'static str {
        match self {
            Strategy::Ours => "--ours",
            Strategy::Theirs => "--theirs",
        }
    }
}

impl FromStr for Strategy {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "--ours" | "ours" => Ok(Strategy::Ours),
            "--theirs" | "theirs" => Ok(Strategy::Theirs),
            other => Err(format!("unsupported strategy: {other}")),
        }
    }
}

impl Resolver for OursResolver {
    fn select<'a>(&self, conflict: &'a ConflictNode) -> SelectedLane<'a> {
        SelectedLane {
            name: LaneName::Ours,
            nodes: &conflict.ours,
        }
    }
}

impl Resolver for TheirsResolver {
    fn select<'a>(&self, conflict: &'a ConflictNode) -> SelectedLane<'a> {
        SelectedLane {
            name: LaneName::Theirs,
            nodes: &conflict.theirs,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::Span;
    use crate::syntax::ast::{CommandHead, ConflictNode, Metadata, RawTextNode};

    fn conflict() -> ConflictNode {
        let span = Span {
            file_id: 0,
            start: 0,
            end: 1,
        };
        ConflictNode {
            command: CommandHead::parse("print"),
            ours: vec![Node::RawText(RawTextNode {
                text: "ours".to_string(),
                span,
            })],
            base: None,
            theirs: vec![Node::RawText(RawTextNode {
                text: "theirs".to_string(),
                span,
            })],
            metadata: Metadata::parse("print"),
            span,
        }
    }

    #[test]
    fn parses_supported_strategy_flags() {
        assert_eq!("--ours".parse::<Strategy>(), Ok(Strategy::Ours));
        assert_eq!("theirs".parse::<Strategy>(), Ok(Strategy::Theirs));
    }

    #[test]
    fn rejects_unsupported_strategy() {
        assert_eq!(
            "--union".parse::<Strategy>(),
            Err("unsupported strategy: --union".to_string())
        );
    }

    #[test]
    fn ours_resolver_selects_ours_lane() {
        let conflict = conflict();
        let selection = OursResolver.select(&conflict);

        assert_eq!(selection.name, LaneName::Ours);
        assert_eq!(selection.nodes.len(), 1);
    }

    #[test]
    fn theirs_resolver_selects_theirs_lane() {
        let conflict = conflict();
        let selection = TheirsResolver.select(&conflict);

        assert_eq!(selection.name, LaneName::Theirs);
        assert_eq!(selection.nodes.len(), 1);
    }

    #[test]
    fn strategy_returns_resolver() {
        let conflict = conflict();
        let resolver = Strategy::Ours.resolver();

        assert_eq!(resolver.select(&conflict).name, LaneName::Ours);
    }
}
