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
    about = "Transitively reduced history-inclusion graph for Git branch refs"
)]
pub struct Cli {
    /// Skip git fetch
    #[arg(long, alias = "offline", global = true)]
    pub no_fetch: bool,

    /// Remote to fetch before reading all refs
    #[arg(long, global = true, default_value = "origin")]
    pub remote: String,

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

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Build a factual graph (the default command)
    Report,
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
        .about("Транзитивно сокращённый граф включения историй веток Git")
        .mut_arg("no_fetch", |arg| {
            russian_option(arg).help("Пропустить git fetch")
        })
        .mut_arg("remote", |arg| {
            russian_option(arg).help("Удалённый репозиторий для получения данных")
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
        .mut_subcommand("report", |sub| {
            localize_russian_command(sub).about("Построить фактический граф (команда по умолчанию)")
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
