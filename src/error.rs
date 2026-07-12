use std::{io, path::PathBuf, process::ExitStatus};

use crate::i18n::{Language, LanguageSaveError};

#[derive(Debug, thiserror::Error)]
pub enum ConfigIssue {
    #[error("unsupported version {0}; expected version = 1")]
    UnsupportedVersion(u32),
    #[error("unexpected git rev-list response: {0}")]
    UnexpectedRevList(String),
    #[error("branch `{0}` was not found")]
    RefNotFound(String),
    #[error("name `{0}` is ambiguous; use a full ref name")]
    AmbiguousRef(String),
    #[error("root branch `{0}` is outside the selected report scope")]
    RootExcluded(String),
    #[error("relationship `{child} = {parent}` uses a remote outside the selected scope")]
    EdgeOutOfScope { child: String, parent: String },
    #[error("ref `{0}` was not found")]
    MissingRef(String),
    #[error("the root branch cannot have a parent")]
    RootHasParent,
    #[error("branch `{0}` is hidden")]
    HiddenBranch(String),
    #[error("parent `{parent}` for `{child}` is missing")]
    MissingParent { parent: String, child: String },
    #[error("a cycle involving `{0}` was detected")]
    Cycle(String),
    #[error("specify a parent or use --unset")]
    MissingParentArgument,
    #[error("remote `{0}` is unavailable")]
    RemoteUnavailable(String),
    #[error("`{parent}` is not an ancestor of `{child}`")]
    NotAncestor { parent: String, child: String },
}

#[derive(Debug, thiserror::Error)]
pub enum LineageError {
    #[error("the command was run outside a Git repository")]
    NotRepository,

    #[error("Git command `{command}` failed ({status}): {message}")]
    Git {
        command: String,
        status: ExitStatus,
        message: String,
    },

    #[error("could not run Git: {0}")]
    GitIo(#[source] io::Error),

    #[error("invalid configuration: {0}")]
    Config(#[from] ConfigIssue),

    #[error("configuration file already exists: {0}; use --force to overwrite it")]
    ConfigExists(PathBuf),

    #[error("could not read {path}: {source}")]
    Read { path: PathBuf, source: io::Error },

    #[error("could not write {path}: {source}")]
    Write { path: PathBuf, source: io::Error },

    #[error("could not parse {path}: {source}")]
    Toml {
        path: PathBuf,
        source: Box<toml::de::Error>,
    },

    #[error("could not serialize configuration: {0}")]
    TomlSerialize(#[from] Box<toml::ser::Error>),

    #[error("could not generate JSON: {0}")]
    Json(#[from] serde_json::Error),

    #[error("could not save language: {0}")]
    LanguageSave(#[from] LanguageSaveError),
}

impl LineageError {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::NotRepository => 3,
            Self::Config(_)
            | Self::ConfigExists(_)
            | Self::Read { .. }
            | Self::Toml { .. }
            | Self::TomlSerialize(_) => 2,
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
            Self::Git {
                command,
                status,
                message,
            } => format!("Git-команда `{command}` завершилась с ошибкой ({status}): {message}"),
            Self::GitIo(error) => format!("не удалось запустить Git: {error}"),
            Self::Config(issue) => format!("некорректная конфигурация: {}", issue.russian()),
            Self::ConfigExists(path) => format!(
                "файл конфигурации уже существует: {}; используйте --force для перезаписи",
                path.display()
            ),
            Self::Read { path, source } => {
                format!("не удалось прочитать {}: {source}", path.display())
            }
            Self::Write { path, source } => {
                format!("не удалось записать {}: {source}", path.display())
            }
            Self::Toml { path, source } => {
                format!("не удалось разобрать {}: {source}", path.display())
            }
            Self::TomlSerialize(error) => {
                format!("не удалось сериализовать конфигурацию: {error}")
            }
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

impl ConfigIssue {
    fn russian(&self) -> String {
        match self {
            Self::UnsupportedVersion(version) => {
                format!("неподдерживаемая версия {version}; ожидается version = 1")
            }
            Self::UnexpectedRevList(text) => {
                format!("неожиданный ответ git rev-list: {text}")
            }
            Self::RefNotFound(name) => format!("ветка `{name}` не найдена"),
            Self::AmbiguousRef(name) => {
                format!("имя `{name}` неоднозначно; укажите полное имя ссылки")
            }
            Self::RootExcluded(root) => {
                format!("корневая ветка `{root}` исключена из отчёта")
            }
            Self::EdgeOutOfScope { child, parent } => {
                format!(
                    "связь `{child} = {parent}` использует remote за пределами выбранной области"
                )
            }
            Self::MissingRef(id) => format!("ссылка `{id}` не найдена"),
            Self::RootHasParent => "корневая ветка не может иметь родителя".into(),
            Self::HiddenBranch(branch) => format!("ветка `{branch}` скрыта"),
            Self::MissingParent { parent, child } => {
                format!("родитель `{parent}` для `{child}` отсутствует")
            }
            Self::Cycle(branch) => format!("обнаружен цикл с участием `{branch}`"),
            Self::MissingParentArgument => "укажите родителя или используйте --unset".into(),
            Self::RemoteUnavailable(remote) => format!("remote `{remote}` недоступен"),
            Self::NotAncestor { parent, child } => {
                format!("`{parent}` не является предком `{child}`")
            }
        }
    }
}

pub type Result<T> = std::result::Result<T, LineageError>;
