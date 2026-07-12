use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    env,
    io::{self, IsTerminal},
    path::Path,
};

use crate::{
    cli::{Cli, Command, OutputFormat, SummaryMode},
    config::{config_path, LineageConfig},
    discover,
    error::{ConfigIssue, LineageError, Result},
    git::GitRepo,
    graph::BranchGraph,
    i18n::Language,
    insights,
    refs::RefSet,
    relation::is_ancestor,
    render::{self, FetchInfo, FetchStatus},
};

pub fn run(cli: Cli, language: Language) -> Result<()> {
    if let Some(Command::Language { language: selected }) = cli.command.as_ref() {
        return language_command(language, *selected);
    }
    let cwd = env::current_dir().map_err(LineageError::GitIo)?;
    let repo = GitRepo::discover(&cwd)?;
    let path = config_path(repo.root(), cli.config.as_ref());

    match cli.command.as_ref() {
        Some(Command::Init { force }) => init(&repo, &path, &cli, *force, language),
        Some(Command::Discover { write }) => run_discover(&repo, &path, &cli, *write, language),
        Some(Command::Parent {
            child,
            parent,
            unset,
        }) => set_parent(&repo, &path, child, parent.as_deref(), *unset, language),
        Some(Command::Check) => check(&repo, &path, &cli, language),
        Some(Command::Report) | None => build_report(&repo, &path, &cli, language),
        Some(Command::Language { .. }) => {
            unreachable!("language command handled before repo lookup")
        }
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

fn init(repo: &GitRepo, path: &Path, cli: &Cli, force: bool, language: Language) -> Result<()> {
    let refs = RefSet::read(repo)?;
    let current = repo.current_branch()?;
    let config = LineageConfig {
        root: cli
            .root
            .clone()
            .unwrap_or_else(|| choose_default_root(&refs, current.as_deref())),
        remote: cli.remote.clone().unwrap_or_else(|| "origin".into()),
        ..LineageConfig::default()
    };
    config.save(path, force)?;
    if language.is_ru() {
        println!("Создан файл: {}", path.display());
    } else {
        println!("Created file: {}", path.display());
    }
    Ok(())
}

fn run_discover(
    repo: &GitRepo,
    path: &Path,
    cli: &Cli,
    write: bool,
    language: Language,
) -> Result<()> {
    let mut config = LineageConfig::load(path)?;
    let remote = cli.remote.clone().unwrap_or_else(|| config.remote.clone());
    let fetch = maybe_fetch(repo, &remote, config.fetch && !cli.no_fetch, language);
    if matches!(fetch.status, FetchStatus::Failed) {
        print_fetch_failure(&remote, fetch.reason.as_deref(), language);
    }
    let refs = RefSet::read(repo)?;
    let current = repo.current_branch()?;
    let root = cli.root.clone().unwrap_or_else(|| {
        if path.exists() {
            config.root.clone()
        } else {
            choose_default_root(&refs, current.as_deref())
        }
    });
    config.root = root.clone();
    config.remote = remote.clone();
    let discovery = discover::discover(repo, &refs, config, &root, &remote, cli.all_remotes)?;
    let toml = toml::to_string_pretty(&discovery.config).map_err(Box::new)?;
    print!("{toml}");
    if !discovery.ambiguous.is_empty() {
        if language.is_ru() {
            println!("\nНе удалось однозначно определить родителей для веток:\n");
        } else {
            println!("\nParents could not be determined unambiguously for:\n");
        }
        for item in &discovery.ambiguous {
            let separator = if language.is_ru() { " и " } else { " and " };
            if language.is_ru() {
                println!(
                    "- {}: возможные родители {}",
                    item.branch,
                    item.candidates.join(separator)
                );
            } else {
                println!(
                    "- {}: possible parents are {}",
                    item.branch,
                    item.candidates.join(separator)
                );
            }
        }
    }
    if !discovery.unplaced.is_empty() {
        if language.is_ru() {
            println!("\nНеразмещённые ветки:\n");
        } else {
            println!("\nUnplaced branches:\n");
        }
        for branch in &discovery.unplaced {
            println!("- {branch}");
        }
    }
    if write {
        discovery.config.save(path, true)?;
        if language.is_ru() {
            eprintln!("Конфигурация записана в {}", path.display());
        } else {
            eprintln!("Configuration written to {}", path.display());
        }
    }
    Ok(())
}

fn set_parent(
    repo: &GitRepo,
    path: &Path,
    child: &str,
    parent: Option<&str>,
    unset: bool,
    language: Language,
) -> Result<()> {
    let mut config = LineageConfig::load(path)?;
    let refs = RefSet::read(repo)?;
    let child_id = refs.resolve(child)?;
    config
        .parents
        .retain(|name, _| refs.resolve(name).ok().as_deref() != Some(&child_id));
    if !unset {
        let parent = parent.ok_or(ConfigIssue::MissingParentArgument)?;
        refs.resolve(parent)?;
        config.parents.insert(child.to_owned(), parent.to_owned());
    }
    config.save(path, true)?;
    if language.is_ru() {
        println!("Файл обновлён: {}", path.display());
    } else {
        println!("Updated file: {}", path.display());
    }
    Ok(())
}

fn check(repo: &GitRepo, path: &Path, cli: &Cli, language: Language) -> Result<()> {
    let config = LineageConfig::load(path)?;
    let remote = cli.remote.as_deref().unwrap_or(&config.remote);
    if !repo.remote_exists(remote)? {
        return Err(ConfigIssue::RemoteUnavailable(remote.into()).into());
    }
    let refs = RefSet::read(repo)?;
    let root = cli.root.as_deref().unwrap_or(&config.root);
    let root_id = refs.resolve(root)?;
    validate_config_cycles(&refs, &config)?;
    reject_root_parent(&refs, &config, &root_id)?;
    for (child_name, parent_name) in &config.parents {
        let child = refs.resolve(child_name)?;
        let parent = refs.resolve(parent_name)?;
        if !is_ancestor(repo, &parent, &child)? {
            return Err(ConfigIssue::NotAncestor {
                parent: parent_name.clone(),
                child: child_name.clone(),
            }
            .into());
        }
    }
    // Building also validates collapsed aliases and every final graph edge.
    BranchGraph::build(
        repo,
        &refs,
        &config,
        root,
        remote,
        cli.all_remotes,
        cli.show_synced_remotes,
        repo.current_branch()?.as_deref(),
    )?;
    if language.is_ru() {
        println!("Конфигурация корректна.");
    } else {
        println!("Configuration is valid.");
    }
    Ok(())
}

fn build_report(repo: &GitRepo, path: &Path, cli: &Cli, language: Language) -> Result<()> {
    let mut config = LineageConfig::load(path)?;
    validate_config_cycles_names(&config)?;
    let remote = cli.remote.clone().unwrap_or_else(|| config.remote.clone());
    let fetch = maybe_fetch(repo, &remote, config.fetch && !cli.no_fetch, language);
    let refs = RefSet::read(repo)?;
    validate_config_cycles(&refs, &config)?;
    let current = repo.current_branch()?;
    let root = cli.root.clone().unwrap_or_else(|| {
        if path.exists() {
            config.root.clone()
        } else {
            choose_default_root(&refs, current.as_deref())
        }
    });
    let summary = cli.summary.unwrap_or(config.display.summary);
    let format = cli.format.unwrap_or_default();
    if format == OutputFormat::Json && matches!(fetch.status, FetchStatus::Failed) {
        print_fetch_failure(&remote, fetch.reason.as_deref(), language);
    }
    if summary == SummaryMode::Full {
        config.display.show_remote_only = true;
    }

    let configured_children: BTreeSet<_> = config
        .parents
        .keys()
        .map(|name| refs.resolve(name))
        .collect::<Result<_>>()?;
    let root_id = refs.resolve(&root)?;
    reject_root_parent(&refs, &config, &root_id)?;
    let needs_discovery = refs.refs.values().any(|item| {
        item.kind == crate::refs::RefKind::Local
            && item.full_name != root_id
            && !configured_children.contains(&item.full_name)
    });
    let mut automatic = Vec::new();
    if needs_discovery {
        let discovered =
            discover::discover(repo, &refs, config.clone(), &root, &remote, cli.all_remotes)?;
        for choice in &discovered.chosen {
            if configured_children.contains(&choice.child_id) {
                continue;
            }
            let entry = discovered
                .config
                .parents
                .iter()
                .find_map(|(child, parent)| {
                    (refs.resolve(child).ok().as_deref() == Some(&choice.child_id))
                        .then(|| (child.clone(), parent.clone()))
                });
            if let Some((child, parent)) = entry {
                config.parents.insert(child, parent);
                automatic.push(choice.clone());
            }
        }
        validate_config_cycles(&refs, &config)?;
    }

    let graph = BranchGraph::build(
        repo,
        &refs,
        &config,
        &root,
        &remote,
        cli.all_remotes,
        cli.show_synced_remotes,
        current.as_deref(),
    )?;
    let mut report_insights = insights::build(repo, &refs, &graph, summary)?;
    if summary == SummaryMode::Full {
        report_insights.extend(
            automatic
                .into_iter()
                .map(|choice| insights::Insight::AutoParent {
                    branch: choice.child,
                    parent: choice.parent,
                    distance: choice.distance,
                }),
        );
    }
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
            "История ограничена из-за неполного (shallow) клона; связи веток могут быть определены неточно.".into()
        } else {
            "Repository history is limited by a shallow clone; branch relationships may be inaccurate.".into()
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
                summary == SummaryMode::Full,
                match cli.color.unwrap_or_default() {
                    crate::cli::ColorMode::Always => true,
                    crate::cli::ColorMode::Never => false,
                    crate::cli::ColorMode::Auto => io::stdout().is_terminal(),
                },
                language,
            )
        ),
        OutputFormat::Json => println!(
            "{}",
            render::json::render(
                repo.root(),
                current.as_deref(),
                &graph,
                &fetch,
                &report_insights,
                &warnings,
            )?
        ),
    }
    Ok(())
}

fn maybe_fetch(repo: &GitRepo, remote: &str, enabled: bool, language: Language) -> FetchInfo {
    if !enabled {
        return FetchInfo {
            remote: remote.into(),
            status: FetchStatus::Skipped,
            reason: None,
        };
    }
    match repo.fetch(remote) {
        Ok(()) => FetchInfo {
            remote: remote.into(),
            status: FetchStatus::Updated,
            reason: None,
        },
        Err(error) => FetchInfo {
            remote: remote.into(),
            status: FetchStatus::Failed,
            reason: Some(fetch_reason(&error, language)),
        },
    }
}

fn fetch_reason(error: &LineageError, language: Language) -> String {
    match error {
        LineageError::Git { message, .. } => message.clone(),
        _ if language.is_ru() => "не удалось выполнить git fetch".into(),
        _ => "could not run git fetch".into(),
    }
}

fn choose_default_root(refs: &RefSet, current: Option<&str>) -> String {
    for candidate in [Some("main"), Some("master"), current]
        .into_iter()
        .flatten()
    {
        if refs.resolve(candidate).is_ok() {
            return candidate.to_owned();
        }
    }
    refs.refs
        .values()
        .find(|r| r.kind == crate::refs::RefKind::Local)
        .map(|r| r.name.clone())
        .unwrap_or_else(|| "main".into())
}

fn validate_config_cycles(refs: &RefSet, config: &LineageConfig) -> Result<()> {
    let mut parents = BTreeMap::new();
    for (child, parent) in &config.parents {
        parents.insert(refs.resolve(child)?, refs.resolve(parent)?);
    }
    detect_parent_cycle(&parents)
}

fn reject_root_parent(refs: &RefSet, config: &LineageConfig, root_id: &str) -> Result<()> {
    for child in config.parents.keys() {
        if refs.resolve(child)?.as_str() == root_id {
            return Err(ConfigIssue::RootHasParent.into());
        }
    }
    Ok(())
}

fn validate_config_cycles_names(config: &LineageConfig) -> Result<()> {
    detect_parent_cycle(&config.parents)
}

fn detect_parent_cycle(parents: &BTreeMap<String, String>) -> Result<()> {
    for start in parents.keys() {
        let mut seen = HashSet::new();
        let mut current = start.as_str();
        while let Some(parent) = parents.get(current) {
            if !seen.insert(current.to_owned()) {
                return Err(ConfigIssue::Cycle(current.into()).into());
            }
            current = parent;
        }
    }
    Ok(())
}

fn print_fetch_failure(remote: &str, reason: Option<&str>, language: Language) {
    let reason = reason.unwrap_or(if language.is_ru() {
        "неизвестная ошибка"
    } else {
        "unknown error"
    });
    if language.is_ru() {
        eprintln!(
            "Не удалось получить данные из {remote}; использованы сохранённые ссылки удалённых веток. Причина: {reason}"
        );
    } else {
        eprintln!(
            "Could not fetch {remote}; cached remote-tracking refs were used. Reason: {reason}"
        );
    }
}
