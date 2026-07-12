use serde::Serialize;

use crate::{
    error::{ConfigIssue, LineageError, Result},
    git::GitRepo,
    i18n::{counted_commits, Language},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Relation {
    pub ahead: u64,
    pub behind: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationKind {
    Synced,
    Ahead(u64),
    Behind(u64),
    Diverged { behind: u64, ahead: u64 },
}

impl Relation {
    pub fn between(repo: &GitRepo, parent: &str, child: &str) -> Result<Self> {
        let range = format!("{parent}...{child}");
        let text = repo.checked_text(["rev-list", "--left-right", "--count", &range])?;
        let mut parts = text.split_whitespace();
        let behind = parts.next().and_then(|v| v.parse().ok());
        let ahead = parts.next().and_then(|v| v.parse().ok());
        match (behind, ahead) {
            (Some(behind), Some(ahead)) => Ok(Self { ahead, behind }),
            _ => Err(ConfigIssue::UnexpectedRevList(text).into()),
        }
    }

    pub fn classify(self) -> RelationKind {
        match (self.behind, self.ahead) {
            (0, 0) => RelationKind::Synced,
            (0, ahead) => RelationKind::Ahead(ahead),
            (behind, 0) => RelationKind::Behind(behind),
            (behind, ahead) => RelationKind::Diverged { behind, ahead },
        }
    }

    pub fn compact(self, parent: &str, language: Language) -> String {
        match (self.classify(), language) {
            (RelationKind::Synced, _) => String::new(),
            (RelationKind::Ahead(count), Language::En) => format!("+{count} from {parent}"),
            (RelationKind::Behind(count), Language::En) => {
                format!("{} behind {parent}", counted_commits(language, count))
            }
            (RelationKind::Diverged { behind, ahead }, Language::En) => {
                format!("−{behind}/+{ahead} relative to {parent}")
            }
            (RelationKind::Ahead(count), Language::Ru) => format!("+{count} от {parent}"),
            (RelationKind::Behind(count), Language::Ru) => format!(
                "отстаёт от {parent} на {}",
                counted_commits(language, count)
            ),
            (RelationKind::Diverged { behind, ahead }, Language::Ru) => {
                format!("−{behind}/+{ahead} относительно {parent}")
            }
        }
    }

    pub fn between_many(repo: &GitRepo, pairs: &[(String, String)]) -> Result<Vec<Self>> {
        if pairs.is_empty() {
            return Ok(Vec::new());
        }
        let worker_count = pairs.len().min(8);
        let chunk_size = pairs.len().div_ceil(worker_count);
        let batches = std::thread::scope(|scope| {
            let mut handles = Vec::new();
            for (chunk_index, chunk) in pairs.chunks(chunk_size).enumerate() {
                handles.push(scope.spawn(move || {
                    chunk
                        .iter()
                        .enumerate()
                        .map(|(offset, (parent, child))| {
                            (
                                chunk_index * chunk_size + offset,
                                Self::between(repo, parent, child),
                            )
                        })
                        .collect::<Vec<_>>()
                }));
            }
            handles
                .into_iter()
                .map(|handle| handle.join().expect("relation worker panicked"))
                .collect::<Vec<_>>()
        });

        let mut ordered = vec![None; pairs.len()];
        for (index, relation) in batches.into_iter().flatten() {
            ordered[index] = Some(relation?);
        }
        Ok(ordered
            .into_iter()
            .map(|relation| relation.expect("relation worker omitted a result"))
            .collect())
    }
}

pub fn is_ancestor(repo: &GitRepo, parent: &str, child: &str) -> Result<bool> {
    let output = repo.output(["merge-base", "--is-ancestor", parent, child])?;
    match output.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => Err(LineageError::Git {
            command: format!("git merge-base --is-ancestor {parent} {child}"),
            status: output.status,
            message: "could not compare branch history".into(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_relations() {
        assert_eq!(
            Relation {
                ahead: 0,
                behind: 0
            }
            .classify(),
            RelationKind::Synced
        );
        assert_eq!(
            Relation {
                ahead: 4,
                behind: 0
            }
            .classify(),
            RelationKind::Ahead(4)
        );
        assert_eq!(
            Relation {
                ahead: 0,
                behind: 3
            }
            .classify(),
            RelationKind::Behind(3)
        );
        assert_eq!(
            Relation {
                ahead: 7,
                behind: 2
            }
            .classify(),
            RelationKind::Diverged {
                behind: 2,
                ahead: 7
            }
        );
    }
}
