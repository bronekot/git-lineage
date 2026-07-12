use std::{io, process::Command};

use clap::ValueEnum;

const LANGUAGE_KEY: &str = "git-lineage.language";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum Language {
    #[default]
    En,
    Ru,
}

impl Language {
    pub fn load() -> Self {
        let output = Command::new("git")
            .args(["config", "--global", "--get", LANGUAGE_KEY])
            .env("GIT_TERMINAL_PROMPT", "0")
            .output();
        let Ok(output) = output else {
            return Self::En;
        };
        if !output.status.success() {
            return Self::En;
        }
        match String::from_utf8_lossy(&output.stdout).trim() {
            "ru" => Self::Ru,
            _ => Self::En,
        }
    }

    pub fn save(self) -> Result<(), LanguageSaveError> {
        let output = Command::new("git")
            .args([
                "config",
                "--global",
                "--replace-all",
                LANGUAGE_KEY,
                self.code(),
            ])
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .map_err(LanguageSaveError::Io)?;
        if output.status.success() {
            Ok(())
        } else {
            let message = String::from_utf8_lossy(&output.stderr).trim().to_owned();
            Err(LanguageSaveError::Git(if message.is_empty() {
                "Git command failed".into()
            } else {
                message
            }))
        }
    }

    pub const fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Ru => "ru",
        }
    }

    pub const fn is_ru(self) -> bool {
        matches!(self, Self::Ru)
    }
}

pub fn commit_word(language: Language, count: u64) -> &'static str {
    match language {
        Language::En if count == 1 => "commit",
        Language::En => "commits",
        Language::Ru => russian_commit_word(count),
    }
}

pub fn counted_commits(language: Language, count: u64) -> String {
    format!("{count} {}", commit_word(language, count))
}

fn russian_commit_word(count: u64) -> &'static str {
    let last_two = count % 100;
    let last = count % 10;
    if (11..=14).contains(&last_two) {
        "коммитов"
    } else if last == 1 {
        "коммит"
    } else if (2..=4).contains(&last) {
        "коммита"
    } else {
        "коммитов"
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LanguageSaveError {
    #[error("could not run Git: {0}")]
    Io(#[source] io::Error),
    #[error("git config failed: {0}")]
    Git(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commit_forms_are_correct_in_both_languages() {
        for value in [1, 21, 101] {
            assert_eq!(commit_word(Language::Ru, value), "коммит");
        }
        for value in [2, 4, 22] {
            assert_eq!(commit_word(Language::Ru, value), "коммита");
        }
        for value in [0, 5, 11, 14, 25] {
            assert_eq!(commit_word(Language::Ru, value), "коммитов");
        }
        assert_eq!(commit_word(Language::En, 1), "commit");
        assert_eq!(commit_word(Language::En, 0), "commits");
        assert_eq!(commit_word(Language::En, 2), "commits");
    }
}
