use std::{
    env,
    io::{self, IsTerminal},
};

use crate::{
    cli::{Cli, ColorMode, Command, OutputFormat, SummaryMode},
    error::{LineageError, Result},
    git::GitRepo,
    graph::BranchGraph,
    i18n::Language,
    insights,
    refs::RefSet,
    render::{self, FetchInfo, FetchStatus},
};

pub fn run(cli: Cli, language: Language) -> Result<()> {
    if let Some(Command::Language { language: selected }) = cli.command.as_ref() {
        return language_command(language, *selected);
    }
    let cwd = env::current_dir().map_err(LineageError::GitIo)?;
    let repo = GitRepo::discover(&cwd)?;
    match cli.command.as_ref() {
        Some(Command::Report) | None => build_report(&repo, &cli, language),
        Some(Command::Language { .. }) => unreachable!("language handled before repository lookup"),
    }
}

fn language_command(current: Language, selected: Option<Language>) -> Result<()> {
    match selected {
        Some(language) => {
            language.save()?;
            if language.is_ru() {
                println!("Язык сохранён: русский (ru).");
            } else {
                println!("Language saved: English (en).");
            }
        }
        None if current.is_ru() => println!("Язык: русский (ru)."),
        None => println!("Language: English (en)."),
    }
    Ok(())
}

fn build_report(repo: &GitRepo, cli: &Cli, language: Language) -> Result<()> {
    let fetch = maybe_fetch(repo, &cli.remote, !cli.no_fetch);
    let refs = RefSet::read(repo)?;
    let current = repo.current_branch()?;
    let graph = BranchGraph::build(repo, &refs, current.as_deref())?;
    let current = current
        .as_deref()
        .and_then(|name| name.strip_prefix("refs/heads/"));
    let summary = cli.summary.unwrap_or(SummaryMode::Brief);
    let format = cli.format.unwrap_or_default();
    if format == OutputFormat::Json {
        print_fetch_diagnostic(&fetch, language);
    }
    let report_insights = insights::build(&refs, &graph, summary);
    let mut warnings = Vec::new();
    if current.is_none() {
        let oid = repo.head_oid()?;
        warnings.push(if language.is_ru() {
            format!("HEAD отсоединён от ветки и указывает на коммит {oid}; текущая локальная ветка не выбрана.")
        } else {
            format!("HEAD is detached at {oid}; there is no current local branch.")
        });
    }
    if repo.is_shallow()? {
        warnings.push(if language.is_ru() {
            "История ограничена из-за неполного (shallow) клона; отношения refs могут быть определены неточно.".into()
        } else {
            "Repository history is limited by a shallow clone; ref relationships may be inaccurate.".into()
        });
    }

    match format {
        OutputFormat::Text => print!(
            "{}",
            render::text::render(
                &graph,
                &fetch,
                &report_insights,
                &warnings,
                cli.ascii,
                use_color(cli.color.unwrap_or_default()),
                language,
            )
        ),
        OutputFormat::Json => println!(
            "{}",
            render::json::render(
                repo.root(),
                current,
                &graph,
                &fetch,
                &report_insights,
                &warnings,
            )?
        ),
    }
    Ok(())
}

fn use_color(mode: ColorMode) -> bool {
    match mode {
        ColorMode::Always => true,
        ColorMode::Never => false,
        // https://no-color.org: any non-empty NO_COLOR disables automatic color.
        ColorMode::Auto => {
            io::stdout().is_terminal()
                && env::var_os("NO_COLOR").is_none_or(|value| value.is_empty())
        }
    }
}

fn maybe_fetch(repo: &GitRepo, remote: &str, enabled: bool) -> FetchInfo {
    let info = |status, reason| FetchInfo {
        remote: remote.into(),
        status,
        reason,
    };
    if !enabled {
        return info(FetchStatus::Skipped, None);
    }
    match repo.has_remote(remote) {
        Ok(true) => {}
        Ok(false) => return info(FetchStatus::MissingRemote, None),
        Err(error) => return info(FetchStatus::Failed, Some(fetch_failure_reason(&error))),
    }
    match repo.fetch(remote) {
        Ok(()) => info(FetchStatus::Updated, None),
        Err(error) => info(FetchStatus::Failed, Some(fetch_failure_reason(&error))),
    }
}

fn print_fetch_diagnostic(fetch: &FetchInfo, language: Language) {
    let remote = &fetch.remote;
    match (&fetch.status, language.is_ru()) {
        (FetchStatus::Failed, true) => {
            eprintln!("Не удалось получить данные из {remote}; используются сохранённые refs.");
            if let Some(reason) = &fetch.reason {
                eprintln!("Причина: {reason}");
            }
        }
        (FetchStatus::Failed, false) => {
            eprintln!("Could not fetch {remote}; using cached refs.");
            if let Some(reason) = &fetch.reason {
                eprintln!("Reason: {reason}");
            }
        }
        (FetchStatus::MissingRemote, true) => {
            eprintln!("Удалённый репозиторий {remote} не настроен; используются локальные refs.");
        }
        (FetchStatus::MissingRemote, false) => {
            eprintln!("Remote {remote} is not configured; using local refs.");
        }
        _ => {}
    }
}

fn fetch_failure_reason(error: &LineageError) -> String {
    match error {
        LineageError::Git { message, .. } => message.clone(),
        _ => "could not run git fetch".into(),
    }
}
