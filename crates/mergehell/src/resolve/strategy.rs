use std::str::FromStr;

use crate::syntax::ast::{ConflictNode, Node};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Strategy {
    Ours,
    Theirs,
    Base,
    Union,
    Random,
    Git,
    Blame,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LaneName {
    Ours,
    Base,
    Theirs,
}

#[derive(Debug, Eq, PartialEq)]
pub struct SelectedLane<'a> {
    pub name: LaneName,
    pub nodes: &'a [Node],
}

pub trait Resolver {
    fn select<'a>(&self, conflict: &'a ConflictNode) -> Option<SelectedLane<'a>>;
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OursResolver;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BaseResolver;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TheirsResolver;

impl Strategy {
    pub fn flag(self) -> &'static str {
        match self {
            Strategy::Ours => "--ours",
            Strategy::Theirs => "--theirs",
            Strategy::Base => "--base",
            Strategy::Union => "--union",
            Strategy::Random => "--random",
            Strategy::Git => "--git",
            Strategy::Blame => "--blame",
        }
    }
}

impl FromStr for Strategy {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "--ours" | "ours" => Ok(Strategy::Ours),
            "--theirs" | "theirs" => Ok(Strategy::Theirs),
            "--base" | "base" => Ok(Strategy::Base),
            "--union" | "union" => Ok(Strategy::Union),
            "--random" | "random" => Ok(Strategy::Random),
            "--git" | "git" => Ok(Strategy::Git),
            "--blame" | "blame" => Ok(Strategy::Blame),
            other => Err(format!("unsupported strategy: {other}")),
        }
    }
}

impl Resolver for OursResolver {
    fn select<'a>(&self, conflict: &'a ConflictNode) -> Option<SelectedLane<'a>> {
        Some(SelectedLane {
            name: LaneName::Ours,
            nodes: &conflict.ours,
        })
    }
}

impl Resolver for BaseResolver {
    fn select<'a>(&self, conflict: &'a ConflictNode) -> Option<SelectedLane<'a>> {
        conflict.base.as_ref().map(|base| SelectedLane {
            name: LaneName::Base,
            nodes: &base.items,
        })
    }
}

impl Resolver for TheirsResolver {
    fn select<'a>(&self, conflict: &'a ConflictNode) -> Option<SelectedLane<'a>> {
        Some(SelectedLane {
            name: LaneName::Theirs,
            nodes: &conflict.theirs,
        })
    }
}

pub fn lanes_in_order(conflict: &ConflictNode) -> Vec<SelectedLane<'_>> {
    let mut lanes = vec![OursResolver.select(conflict).expect("ours lane exists")];
    if let Some(base) = BaseResolver.select(conflict) {
        lanes.push(base);
    }
    lanes.push(TheirsResolver.select(conflict).expect("theirs lane exists"));
    lanes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::Span;
    use crate::syntax::ast::{CommandHead, ConflictNode, Lane, Metadata, RawTextNode};

    fn span() -> Span {
        Span {
            file_id: 0,
            start: 0,
            end: 1,
        }
    }

    fn conflict(with_base: bool) -> ConflictNode {
        let span = span();
        ConflictNode {
            command: CommandHead::parse("print"),
            ours: vec![Node::RawText(RawTextNode {
                text: "ours".to_string(),
                span,
            })],
            base: with_base.then(|| Lane {
                label: Some("base".to_string()),
                items: vec![Node::RawText(RawTextNode {
                    text: "base".to_string(),
                    span,
                })],
                span,
            }),
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
        assert_eq!("--base".parse::<Strategy>(), Ok(Strategy::Base));
        assert_eq!("union".parse::<Strategy>(), Ok(Strategy::Union));
        assert_eq!("--random".parse::<Strategy>(), Ok(Strategy::Random));
        assert_eq!("git".parse::<Strategy>(), Ok(Strategy::Git));
        assert_eq!("--blame".parse::<Strategy>(), Ok(Strategy::Blame));
    }

    #[test]
    fn rejects_unsupported_strategy() {
        assert_eq!(
            "--manual".parse::<Strategy>(),
            Err("unsupported strategy: --manual".to_string())
        );
    }

    #[test]
    fn single_lane_resolvers_select_expected_lanes() {
        let conflict = conflict(true);

        assert_eq!(OursResolver.select(&conflict).unwrap().name, LaneName::Ours);
        assert_eq!(BaseResolver.select(&conflict).unwrap().name, LaneName::Base);
        assert_eq!(
            TheirsResolver.select(&conflict).unwrap().name,
            LaneName::Theirs
        );
    }

    #[test]
    fn base_resolver_returns_none_without_base_lane() {
        let conflict = conflict(false);

        assert_eq!(BaseResolver.select(&conflict), None);
    }

    #[test]
    fn lanes_in_order_includes_base_when_present() {
        let conflict = conflict(true);
        let lanes = lanes_in_order(&conflict);

        assert_eq!(
            lanes.iter().map(|lane| lane.name).collect::<Vec<_>>(),
            vec![LaneName::Ours, LaneName::Base, LaneName::Theirs]
        );
    }

    #[test]
    fn lanes_in_order_omits_missing_base() {
        let conflict = conflict(false);
        let lanes = lanes_in_order(&conflict);

        assert_eq!(
            lanes.iter().map(|lane| lane.name).collect::<Vec<_>>(),
            vec![LaneName::Ours, LaneName::Theirs]
        );
    }
}
