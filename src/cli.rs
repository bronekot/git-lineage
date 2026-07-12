use std::path::PathBuf;

use clap::{CommandFactory, FromArgMatches, Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};

use crate::i18n::Language;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum SummaryMode {
    None,
    #[default]
    Brief,
    Full,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum ColorMode {
    #[default]
    Auto,
    Always,
    Never,
}

#[derive(Debug, Parser)]
#[command(
    name = "git-lineage",
    version,
    about = "Compact semantic branch trees for Git repositories"
)]
pub struct Cli {
    /// Skip git fetch
    #[arg(long, alias = "offline", global = true)]
    pub no_fetch: bool,

    /// Remote to fetch and display
    #[arg(long, global = true)]
    pub remote: Option<String>,

    /// Root ref for the tree
    #[arg(long, global = true)]
    pub root: Option<String>,

    /// Configuration file path
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,

    /// Insight detail level
    #[arg(long, value_enum, global = true)]
    pub summary: Option<SummaryMode>,

    /// Output format
    #[arg(long, value_enum, global = true)]
    pub format: Option<OutputFormat>,

    /// Color mode
    #[arg(long, value_enum, global = true)]
    pub color: Option<ColorMode>,

    /// Use ASCII tree characters
    #[arg(long, global = true)]
    pub ascii: bool,

    /// Include branches from every remote
    #[arg(long, global = true)]
    pub all_remotes: bool,

    /// Show synced upstream branches
    #[arg(long, global = true)]
    pub show_synced_remotes: bool,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Build a report (the default command)
    Report,
    /// Create .git-lineage.toml
    Init {
        /// Overwrite an existing configuration file
        #[arg(long)]
        force: bool,
    },
    /// Suggest a logical branch structure
    Discover {
        /// Write the discovered relationships to .git-lineage.toml
        #[arg(long)]
        write: bool,
    },
    /// Set or remove a branch parent
    Parent {
        /// Child branch
        child: String,
        /// Logical parent branch
        parent: Option<String>,
        /// Remove the configured parent
        #[arg(long, conflicts_with = "parent")]
        unset: bool,
    },
    /// Validate the configuration
    Check,
    /// Show or save the interface language
    #[command(alias = "lang")]
    Language {
        /// Language to save; omit to show the current language
        #[arg(value_enum)]
        language: Option<Language>,
    },
}

pub fn parse(language: Language) -> Cli {
    let mut command = Cli::command();
    if language.is_ru() {
        command = localize_russian(command);
    }
    Cli::from_arg_matches(&command.get_matches()).expect("Clap matches must produce Cli")
}

fn localize_russian(command: clap::Command) -> clap::Command {
    let mut command = localize_russian_command(command)
        .disable_help_flag(true)
        .disable_version_flag(true)
        .arg(
            clap::Arg::new("help")
                .short('h')
                .long("help")
                .global(true)
                .action(clap::ArgAction::Help)
                .help_heading("Параметры")
                .help("Показать справку"),
        )
        .arg(
            clap::Arg::new("version")
                .short('V')
                .long("version")
                .action(clap::ArgAction::Version)
                .help_heading("Параметры")
                .help("Показать версию"),
        )
        .subcommand_help_heading("Команды")
        .subcommand_value_name("КОМАНДА")
        .about("Компактное семантическое дерево веток Git")
        .mut_arg("no_fetch", |arg| {
            russian_option(arg).help("Пропустить git fetch")
        })
        .mut_arg("remote", |arg| {
            russian_option(arg).help("Удалённый репозиторий для получения данных и отчёта")
        })
        .mut_arg("root", |arg| {
            russian_option(arg).help("Корневая ветка дерева")
        })
        .mut_arg("config", |arg| {
            russian_option(arg).help("Путь к файлу конфигурации")
        })
        .mut_arg("summary", |arg| {
            russian_option(arg)
                .hide_possible_values(true)
                .help("Объём пояснений [значения: none, brief, full]")
        })
        .mut_arg("format", |arg| {
            russian_option(arg)
                .hide_possible_values(true)
                .help("Формат вывода [значения: text, json]")
        })
        .mut_arg("color", |arg| {
            russian_option(arg)
                .hide_possible_values(true)
                .help("Режим цветного вывода [значения: auto, always, never]")
        })
        .mut_arg("ascii", |arg| {
            russian_option(arg).help("Использовать символы дерева из ASCII")
        })
        .mut_arg("all_remotes", |arg| {
            russian_option(arg).help("Учитывать ветки из всех удалённых репозиториев")
        })
        .mut_arg("show_synced_remotes", |arg| {
            russian_option(arg).help("Показывать совпадающие upstream-ветки")
        })
        .mut_subcommand("report", |sub| {
            localize_russian_command(sub).about("Построить отчёт (команда по умолчанию)")
        })
        .mut_subcommand("init", |sub| {
            localize_russian_command(sub)
                .about("Создать .git-lineage.toml")
                .mut_arg("force", |arg| {
                    russian_option(arg).help("Перезаписать существующую конфигурацию")
                })
        })
        .mut_subcommand("discover", |sub| {
            localize_russian_command(sub)
                .about("Предложить логическую структуру веток")
                .mut_arg("write", |arg| {
                    russian_option(arg).help("Записать найденные связи в .git-lineage.toml")
                })
        })
        .mut_subcommand("parent", |sub| {
            localize_russian_command(sub)
                .about("Установить или удалить родителя ветки")
                .mut_arg("child", |arg| russian_argument(arg).help("Дочерняя ветка"))
                .mut_arg("parent", |arg| {
                    russian_argument(arg).help("Логическая родительская ветка")
                })
                .mut_arg("unset", |arg| {
                    russian_option(arg).help("Удалить настроенного родителя")
                })
        })
        .mut_subcommand("check", |sub| {
            localize_russian_command(sub).about("Проверить конфигурацию")
        })
        .mut_subcommand("language", |sub| {
            localize_russian_command(sub)
                .about("Показать или сохранить язык интерфейса")
                .mut_arg("language", |arg| {
                    russian_argument(arg).hide_possible_values(true).help(
                        "Язык для сохранения; без значения — показать текущий [значения: en, ru]",
                    )
                })
        });
    command.build();
    command.mut_subcommand("help", |sub| {
        localize_russian_command(sub).about("Показать справку по команде")
    })
}

const RUSSIAN_HELP_TEMPLATE: &str =
    "{before-help}{about-with-newline}\nИспользование: {usage}\n\n{all-args}{after-help}";

fn localize_russian_command(command: clap::Command) -> clap::Command {
    command.help_template(RUSSIAN_HELP_TEMPLATE)
}

fn russian_option(arg: clap::Arg) -> clap::Arg {
    arg.help_heading("Параметры")
}

fn russian_argument(arg: clap::Arg) -> clap::Arg {
    arg.help_heading("Аргументы")
}
