use std::{io, process::ExitStatus};

use crate::i18n::{Language, LanguageSaveError};

#[derive(Debug, thiserror::Error)]
pub enum LineageError {
    #[error("the command was run outside a Git repository")]
    NotRepository,

    #[error("no local or remote branch refs were found; the repository may have no commits yet")]
    NoRefs,

    #[error("Git command `{command}` failed ({status}): {message}")]
    Git {
        command: String,
        status: ExitStatus,
        message: String,
    },

    #[error("could not run Git: {0}")]
    GitIo(#[source] io::Error),

    #[error("could not generate JSON: {0}")]
    Json(#[from] serde_json::Error),

    #[error("could not save language: {0}")]
    LanguageSave(#[from] LanguageSaveError),
}

impl LineageError {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::NotRepository => 3,
            Self::NoRefs => 2,
            _ => 1,
        }
    }

    pub fn localized(&self, language: Language) -> String {
        let message = if language.is_ru() {
            self.russian_message()
        } else {
            self.to_string()
        };
        if language.is_ru() {
            format!("Ошибка: {message}")
        } else {
            format!("Error: {message}")
        }
    }

    fn russian_message(&self) -> String {
        match self {
            Self::NotRepository => "команда запущена вне Git-репозитория".into(),
            Self::NoRefs => {
                "не найдены локальные или удалённые ветки; возможно, в репозитории ещё нет коммитов"
                    .into()
            }
            Self::Git {
                command,
                status,
                message,
            } => format!("Git-команда `{command}` завершилась с ошибкой ({status}): {message}"),
            Self::GitIo(error) => format!("не удалось запустить Git: {error}"),
            Self::Json(error) => format!("не удалось сформировать JSON: {error}"),
            Self::LanguageSave(LanguageSaveError::Io(error)) => {
                format!("не удалось сохранить язык: не удалось запустить Git: {error}")
            }
            Self::LanguageSave(LanguageSaveError::Git(message)) => format!(
                "не удалось сохранить язык: git config завершился с ошибкой: {}",
                if message == "Git command failed" {
                    "Git не сообщил подробностей"
                } else {
                    message
                }
            ),
        }
    }
}

pub type Result<T> = std::result::Result<T, LineageError>;
