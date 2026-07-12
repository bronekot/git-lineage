use std::collections::BTreeMap;

use unicode_width::UnicodeWidthStr;

use crate::{
    graph::BranchGraph,
    i18n::{commit_word, counted_commits, Language},
    insights::Insight,
    render::{FetchInfo, FetchStatus},
};

#[allow(clippy::too_many_arguments)]
pub fn render(
    graph: &BranchGraph,
    fetch: &FetchInfo,
    insights: &[Insight],
    warnings: &[String],
    ascii: bool,
    include_unplaced: bool,
    color: bool,
    language: Language,
) -> String {
    let mut out = String::new();
    render_header(&mut out, fetch, language);

    let mut lines = Vec::<(String, String)>::new();
    collect_lines(graph, &graph.root, &[], true, ascii, language, &mut lines);
    let width = lines
        .iter()
        .map(|(left, _)| UnicodeWidthStr::width(left.as_str()))
        .max()
        .unwrap_or(0);
    for (left, relation) in lines {
        if relation.is_empty() {
            out.push_str(&left);
        } else {
            let padding = width.saturating_sub(UnicodeWidthStr::width(left.as_str())) + 2;
            out.push_str(&left);
            out.push_str(&" ".repeat(padding));
            out.push_str(&relation);
        }
        out.push('\n');
    }

    let insight_lines = insight_lines(insights, language);
    if !insight_lines.is_empty()
        || !warnings.is_empty()
        || (include_unplaced && !graph.unplaced.is_empty())
    {
        out.push('\n');
    }
    for line in insight_lines {
        out.push_str("- ");
        out.push_str(&line);
        out.push('\n');
    }
    for warning in warnings {
        out.push_str("- ");
        out.push_str(warning);
        out.push('\n');
    }
    if include_unplaced && !graph.unplaced.is_empty() {
        out.push_str(if language.is_ru() {
            "\nНеразмещённые ветки:\n\n"
        } else {
            "\nUnplaced branches:\n\n"
        });
        for branch in &graph.unplaced {
            out.push_str(&format!("- {branch}\n"));
        }
    }
    if color {
        if let Some(current) = graph.nodes.values().find(|node| node.current) {
            let marker = current_marker(language);
            let plain = format!("{} {marker}", current.name);
            let colored = format!("\u{1b}[1;36m{}\u{1b}[0m {marker}", current.name);
            return out.replacen(&plain, &colored, 1);
        }
    }
    out
}

fn render_header(out: &mut String, fetch: &FetchInfo, language: Language) {
    match (fetch.status.clone(), language) {
        (FetchStatus::Updated, Language::En) => {
            out.push_str(&format!("Fetched {}; branch graph:\n\n", fetch.remote));
        }
        (FetchStatus::Skipped, Language::En) => {
            out.push_str("Branch graph from local data:\n\n");
        }
        (FetchStatus::Failed, Language::En) => {
            out.push_str(&format!(
                "Could not fetch {}; branch graph uses cached data:\n",
                fetch.remote
            ));
            if let Some(reason) = &fetch.reason {
                out.push_str(&format!("Reason: {reason}\n"));
            }
            out.push('\n');
        }
        (FetchStatus::Updated, Language::Ru) => {
            out.push_str(&format!(
                "Данные из {} получены; граф веток:\n\n",
                fetch.remote
            ));
        }
        (FetchStatus::Skipped, Language::Ru) => {
            out.push_str("Граф веток по локальным данным:\n\n");
        }
        (FetchStatus::Failed, Language::Ru) => {
            out.push_str(&format!(
                "Не удалось получить данные из {}; граф построен по сохранённым данным:\n",
                fetch.remote
            ));
            if let Some(reason) = &fetch.reason {
                out.push_str(&format!("Причина: {reason}\n"));
            }
            out.push('\n');
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn collect_lines(
    graph: &BranchGraph,
    id: &str,
    ancestors_last: &[bool],
    is_last: bool,
    ascii: bool,
    language: Language,
    lines: &mut Vec<(String, String)>,
) {
    let node = &graph.nodes[id];
    let mut left = String::new();
    if !ancestors_last.is_empty() {
        for &ancestor_last in &ancestors_last[..ancestors_last.len() - 1] {
            left.push_str(if ancestor_last {
                "   "
            } else if ascii {
                "|  "
            } else {
                "│  "
            });
        }
        left.push_str(if is_last {
            if ascii {
                "`- "
            } else {
                "└─ "
            }
        } else if ascii {
            "+- "
        } else {
            "├─ "
        });
    }
    left.push_str(&node.name);
    if node.current {
        left.push(' ');
        left.push_str(current_marker(language));
    }
    let relation = node
        .relation()
        .and_then(|relation| {
            let parent = node.parent.as_ref()?;
            Some(relation.compact(&graph.nodes[parent].name, language))
        })
        .unwrap_or_default();
    lines.push((left, relation));

    let children = graph.children(id);
    for (index, child) in children.iter().enumerate() {
        let child_last = index + 1 == children.len();
        let mut next = ancestors_last.to_vec();
        next.push(child_last);
        collect_lines(graph, &child.id, &next, child_last, ascii, language, lines);
    }
}

fn current_marker(language: Language) -> &'static str {
    if language.is_ru() {
        "(текущая)"
    } else {
        "(current)"
    }
}

fn insight_lines(insights: &[Insight], language: Language) -> Vec<String> {
    let mut lines = Vec::new();
    let mut root_groups: BTreeMap<(&str, u64), Vec<&str>> = BTreeMap::new();
    let mut synced: BTreeMap<&str, Vec<&str>> = BTreeMap::new();

    for insight in insights {
        match insight {
            Insight::RootBehind {
                root,
                branch,
                count,
            } => root_groups.entry((root, *count)).or_default().push(branch),
            Insight::SyncedUpstream { branch, upstream } => {
                let remote = upstream
                    .split_once('/')
                    .map(|(remote, _)| remote)
                    .unwrap_or("remote");
                synced.entry(remote).or_default().push(branch);
            }
            Insight::AutoParent {
                branch,
                parent,
                distance,
            } => lines.push(if language.is_ru() {
                format!(
                    "Вероятный родитель ветки {branch} — {parent}: ближайший строгий предок, расстояние — {}.",
                    counted_commits(language, *distance)
                )
            } else {
                format!(
                    "Likely parent of {branch}: {parent} (nearest strict ancestor, {} away).",
                    counted_commits(language, *distance)
                )
            }),
            Insight::TrackingBehind {
                branch,
                upstream,
                count,
            } => lines.push(if language.is_ru() {
                format!(
                    "Локальная ветка {branch} отстаёт от {upstream} на {}.",
                    counted_commits(language, *count)
                )
            } else {
                format!(
                    "Local branch {branch} is {} behind {upstream}.",
                    counted_commits(language, *count)
                )
            }),
            Insight::TrackingAhead {
                branch,
                upstream,
                count,
            } => lines.push(if language.is_ru() {
                format!(
                    "Локальная ветка {branch} опережает {upstream} на {}.",
                    counted_commits(language, *count)
                )
            } else {
                format!(
                    "Local branch {branch} is {} ahead of {upstream}.",
                    counted_commits(language, *count)
                )
            }),
            Insight::TrackingDiverged {
                branch,
                upstream,
                local_count,
                remote_count,
            } => lines.push(if language.is_ru() {
                format!(
                    "{branch} и {upstream} разошлись: в {branch} есть {}, а в {upstream} — {}.",
                    unique_commits_ru(*local_count),
                    unique_commits_ru(*remote_count)
                )
            } else {
                format!(
                    "{branch} and {upstream} diverged: {branch} has {}, while {upstream} has {}.",
                    unique_commits_en(*local_count),
                    unique_commits_en(*remote_count)
                )
            }),
            Insight::Siblings {
                parent,
                left,
                right,
                left_count,
                right_count,
            } if left_count == right_count => lines.push(if language.is_ru() {
                format!(
                    "{left} и {right} — соседние ветки от {parent}: у каждой есть {}.",
                    per_unique_commits_ru(*left_count)
                )
            } else {
                format!(
                    "{left} and {right} are sibling branches from {parent}: each has {}.",
                    unique_commits_en(*left_count)
                )
            }),
            Insight::Siblings {
                parent,
                left,
                right,
                left_count,
                right_count,
            } => lines.push(if language.is_ru() {
                format!(
                    "{left} и {right} разошлись от {parent}: в {left} есть {}, а в {right} — {}.",
                    unique_commits_ru(*left_count),
                    unique_commits_ru(*right_count)
                )
            } else {
                format!(
                    "{left} and {right} diverged from {parent}: {left} has {}, while {right} has {}.",
                    unique_commits_en(*left_count),
                    unique_commits_en(*right_count)
                )
            }),
        }
    }
    for ((root, count), branches) in root_groups {
        lines.insert(
            0,
            if language.is_ru() {
                format!(
                    "{root} отстаёт от {} на {}.",
                    join_names(&branches, language),
                    counted_commits(language, count)
                )
            } else {
                format!(
                    "{root} is {} behind {}.",
                    counted_commits(language, count),
                    join_names(&branches, language)
                )
            },
        );
    }
    for (remote, branches) in synced {
        if language.is_ru() {
            if branches.len() == 1 {
                lines.push(format!(
                    "{} совпадает с соответствующей веткой на {remote}.",
                    branches[0]
                ));
            } else {
                lines.push(format!(
                    "{} совпадают с соответствующими ветками на {remote}.",
                    join_names(&branches, language)
                ));
            }
        } else if branches.len() == 1 {
            lines.push(format!(
                "{} matches its counterpart on {remote}.",
                branches[0]
            ));
        } else {
            lines.push(format!(
                "{} match their counterparts on {remote}.",
                join_names(&branches, language)
            ));
        }
    }
    lines
}

fn join_names(names: &[&str], language: Language) -> String {
    let conjunction = if language.is_ru() { "и" } else { "and" };
    match names {
        [] => String::new(),
        [one] => (*one).to_owned(),
        [left, right] => format!("{left} {conjunction} {right}"),
        many => format!(
            "{} {conjunction} {}",
            many[..many.len() - 1].join(", "),
            many[many.len() - 1]
        ),
    }
}

fn unique_commits_en(count: u64) -> String {
    format!("{count} unique {}", commit_word(Language::En, count))
}

fn unique_commits_ru(count: u64) -> String {
    if count % 10 == 1 && count % 100 != 11 {
        format!("{count} уникальный коммит")
    } else {
        format!("{count} уникальных {}", commit_word(Language::Ru, count))
    }
}

fn per_unique_commits_ru(count: u64) -> String {
    if count % 10 == 1 && count % 100 != 11 {
        format!("по {count} уникальному коммиту")
    } else {
        format!("по {count} уникальных {}", commit_word(Language::Ru, count))
    }
}
