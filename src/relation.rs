use serde::Serialize;

use crate::i18n::{counted_commits, Language};

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
