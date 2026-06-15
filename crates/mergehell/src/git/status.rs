use std::path::PathBuf;
use std::process::Command;

use crate::resolve::strategy::Strategy;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GitStatusMode {
    Disabled,
    Enabled,
}

impl Default for GitStatusMode {
    fn default() -> Self {
        Self::Disabled
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GitStateSnapshot {
    pub branch: Option<String>,
    pub detached: bool,
    pub dirty: bool,
    pub merge_in_progress: bool,
    pub rebase_in_progress: bool,
    pub bisecting: bool,
    pub inside_work_tree: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitStrategyError {
    pub message: String,
    pub hint: Option<String>,
}

pub trait GitState {
    fn snapshot(&self) -> Result<GitStateSnapshot, GitStrategyError>;
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RealGitState;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FakeGitState {
    snapshot: Result<GitStateSnapshot, GitStrategyError>,
}

impl FakeGitState {
    pub fn new(snapshot: GitStateSnapshot) -> Self {
        Self {
            snapshot: Ok(snapshot),
        }
    }

    pub fn failing(message: impl Into<String>) -> Self {
        Self {
            snapshot: Err(GitStrategyError {
                message: message.into(),
                hint: None,
            }),
        }
    }
}

impl GitState for FakeGitState {
    fn snapshot(&self) -> Result<GitStateSnapshot, GitStrategyError> {
        self.snapshot.clone()
    }
}

impl GitState for RealGitState {
    fn snapshot(&self) -> Result<GitStateSnapshot, GitStrategyError> {
        detect_current_snapshot()
    }
}

pub fn decide_strategy(snapshot: &GitStateSnapshot) -> Result<Strategy, GitStrategyError> {
    if !snapshot.inside_work_tree {
        return Err(GitStrategyError {
            message: "fatal: not a git repository".to_string(),
            hint: Some("hint: use --ours or run inside a work tree".to_string()),
        });
    }
    if snapshot.merge_in_progress {
        return Err(GitStrategyError {
            message: "fatal: merge in progress requires manual resolution".to_string(),
            hint: Some("hint: --manual is not implemented yet".to_string()),
        });
    }
    if snapshot.rebase_in_progress {
        return Ok(Strategy::Theirs);
    }
    if snapshot.dirty {
        return Ok(Strategy::Union);
    }
    if snapshot.detached {
        return Ok(Strategy::Random);
    }
    if snapshot.branch.as_deref() == Some("main") {
        return Ok(Strategy::Ours);
    }
    if snapshot.branch.is_some() {
        return Ok(Strategy::Theirs);
    }

    Err(GitStrategyError {
        message: "fatal: program is clean".to_string(),
        hint: Some("hint: introduce a conflict and try again".to_string()),
    })
}

pub fn strategy_from_git_state(git_state: &dyn GitState) -> Result<Strategy, GitStrategyError> {
    let snapshot = git_state.snapshot()?;
    decide_strategy(&snapshot)
}

pub fn strategy_from_current_repo() -> Result<Strategy, GitStrategyError> {
    strategy_from_git_state(&RealGitState)
}

pub fn runtime_metadata_for_status_line(text: &str) -> Option<(&'static str, String)> {
    let trimmed = text.trim();
    if let Some(branch) = trimmed.strip_prefix("On branch ") {
        Some(("git.branch", branch.to_string()))
    } else if trimmed == "You have unmerged paths." {
        Some(("git.unmerged", "true".to_string()))
    } else if let Some(path) = trimmed.strip_prefix("both modified:") {
        Some(("git.status.both_modified", path.trim().to_string()))
    } else if let Some(path) = trimmed.strip_prefix("deleted by us:") {
        Some(("git.status.deleted_by_us", path.trim().to_string()))
    } else if let Some(path) = trimmed.strip_prefix("deleted by them:") {
        Some(("git.status.deleted_by_them", path.trim().to_string()))
    } else if let Some(path) = trimmed.strip_prefix("added by us:") {
        Some(("git.status.added_by_us", path.trim().to_string()))
    } else if let Some(path) = trimmed.strip_prefix("added by them:") {
        Some(("git.status.added_by_them", path.trim().to_string()))
    } else if trimmed.starts_with("nothing to commit") || trimmed.starts_with("working tree clean")
    {
        Some(("git.clean", "true".to_string()))
    } else {
        None
    }
}

fn detect_current_snapshot() -> Result<GitStateSnapshot, GitStrategyError> {
    let inside = git_output(["rev-parse", "--is-inside-work-tree"])?;
    if inside.trim() != "true" {
        return Ok(GitStateSnapshot::default());
    }

    let branch = git_output(["symbolic-ref", "--short", "-q", "HEAD"])
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let git_dir = git_output(["rev-parse", "--git-dir"])?;
    let git_dir = PathBuf::from(git_dir.trim());
    let status = git_output(["status", "--porcelain"])?;
    let detached = branch.is_none();

    Ok(GitStateSnapshot {
        branch,
        detached,
        dirty: !status.trim().is_empty(),
        merge_in_progress: git_dir.join("MERGE_HEAD").exists(),
        rebase_in_progress: git_dir.join("rebase-merge").exists()
            || git_dir.join("rebase-apply").exists(),
        bisecting: git_dir.join("BISECT_LOG").exists(),
        inside_work_tree: true,
    })
}

fn git_output<const N: usize>(args: [&str; N]) -> Result<String, GitStrategyError> {
    let output = Command::new("git")
        .args(args)
        .output()
        .map_err(|error| GitStrategyError {
            message: format!("fatal: could not run git: {error}"),
            hint: Some("hint: use an explicit strategy such as --ours".to_string()),
        })?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(GitStrategyError {
            message: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            hint: Some("hint: use an explicit strategy such as --ours".to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> GitStateSnapshot {
        GitStateSnapshot {
            inside_work_tree: true,
            branch: Some("main".to_string()),
            ..GitStateSnapshot::default()
        }
    }

    #[test]
    fn git_status_mode_defaults_to_disabled() {
        assert_eq!(GitStatusMode::default(), GitStatusMode::Disabled);
    }

    #[test]
    fn main_branch_prefers_ours() {
        assert_eq!(decide_strategy(&snapshot()), Ok(Strategy::Ours));
    }

    #[test]
    fn feature_branch_prefers_theirs() {
        let mut snapshot = snapshot();
        snapshot.branch = Some("feature/demo".to_string());

        assert_eq!(decide_strategy(&snapshot), Ok(Strategy::Theirs));
    }

    #[test]
    fn rebase_prefers_theirs() {
        let mut snapshot = snapshot();
        snapshot.rebase_in_progress = true;

        assert_eq!(decide_strategy(&snapshot), Ok(Strategy::Theirs));
    }

    #[test]
    fn merge_in_progress_requires_manual_resolution() {
        let mut snapshot = snapshot();
        snapshot.merge_in_progress = true;

        assert_eq!(
            decide_strategy(&snapshot).unwrap_err().message,
            "fatal: merge in progress requires manual resolution"
        );
    }

    #[test]
    fn dirty_tree_prefers_union() {
        let mut snapshot = snapshot();
        snapshot.dirty = true;

        assert_eq!(decide_strategy(&snapshot), Ok(Strategy::Union));
    }

    #[test]
    fn detached_head_prefers_random() {
        let mut snapshot = snapshot();
        snapshot.branch = None;
        snapshot.detached = true;

        assert_eq!(decide_strategy(&snapshot), Ok(Strategy::Random));
    }

    #[test]
    fn non_repository_errors() {
        assert_eq!(
            decide_strategy(&GitStateSnapshot::default())
                .unwrap_err()
                .message,
            "fatal: not a git repository"
        );
    }

    #[test]
    fn fake_git_state_can_drive_decision() {
        let mut snapshot = snapshot();
        snapshot.branch = Some("feature/demo".to_string());
        let fake = FakeGitState::new(snapshot);

        assert_eq!(strategy_from_git_state(&fake), Ok(Strategy::Theirs));
    }

    #[test]
    fn fake_git_state_can_fail() {
        let fake = FakeGitState::failing("fatal: no git");

        assert_eq!(
            strategy_from_git_state(&fake).unwrap_err().message,
            "fatal: no git"
        );
    }

    #[test]
    fn parses_status_lines_into_runtime_metadata() {
        assert_eq!(
            runtime_metadata_for_status_line("On branch main"),
            Some(("git.branch", "main".to_string()))
        );
        assert_eq!(
            runtime_metadata_for_status_line("  both modified:   src/main.mh"),
            Some(("git.status.both_modified", "src/main.mh".to_string()))
        );
        assert_eq!(
            runtime_metadata_for_status_line("working tree clean"),
            Some(("git.clean", "true".to_string()))
        );
        assert_eq!(runtime_metadata_for_status_line("Unmerged paths:"), None);
    }
}
