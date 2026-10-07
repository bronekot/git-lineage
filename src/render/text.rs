use std::collections::{BTreeMap, HashSet};

use unicode_width::UnicodeWidthStr;

use crate::{
    graph::BranchGraph,
    i18n::{commit_word, counted_commits, Language},
    insights::{Insight, Insights},
    relation::Relation,
    render::{FetchInfo, FetchStatus},
};

pub fn render(
    graph: &BranchGraph,
    fetch: &FetchInfo,
    insights: &Insights,
    warnings: &[String],
    ascii: bool,
    color: bool,
    language: Language,
) -> String {
    let mut out = String::new();
    render_header(&mut out, fetch, language);

    let mut lines = Vec::<TreeLine>::new();
    let mut visited = HashSet::new();
    for root in &graph.roots {
        collect_lines(
            graph,
            root,
            None,
            &[],
            true,
            ascii,
            language,
            &mut visited,
            &mut lines,
        );
    }
    let width = lines.iter().map(TreeLine::width).max().unwrap_or(0);
    for line in &lines {
        out.push_str(&line.prefix);
        if color && line.current {
            out.push_str(&format!("\u{1b}[1;36m{}\u{1b}[0m", line.label));
        } else {
            out.push_str(&line.label);
        }
        out.push_str(&line.suffix);
        if !line.relation.is_empty() {
            out.push_str(&" ".repeat(width - line.width() + 2));
            out.push_str(&line.relation);
        }
        out.push('\n');
    }

    let mut insight_lines = insight_lines(&insights.items, language);
    if insights.hidden_diverged > 0 {
        insight_lines.push(hidden_divergence_hint(insights.hidden_diverged, language));
    }
    if !insight_lines.is_empty() || !warnings.is_empty() {
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
    out.push_str(&if language.is_ru() {
        format!(
            "\nСсылок: {}, узлов: {} (объединено совпадающих ссылок: {}), связей после транзитивного сокращения: {}.\n",
            graph.total_refs,
            graph.nodes.len(),
            graph.merged_aliases,
            graph.edge_count
        )
    } else {
        format!(
            "\nRefs: {}, nodes: {} (coincident refs merged: {}), edges after transitive reduction: {}.\n",
            graph.total_refs,
            graph.nodes.len(),
            graph.merged_aliases,
            graph.edge_count
        )
    });
    out
}

struct TreeLine {
    prefix: String,
    label: String,
    suffix: String,
    relation: String,
    current: bool,
}

impl TreeLine {
    fn width(&self) -> usize {
        UnicodeWidthStr::width(self.prefix.as_str())
            + UnicodeWidthStr::width(self.label.as_str())
            + UnicodeWidthStr::width(self.suffix.as_str())
    }
}

fn hidden_divergence_hint(count: usize, language: Language) -> String {
    if language.is_ru() {
        format!("Других расходящихся пар веток: {count}; описать их — --summary full.")
    } else {
        format!("Other diverged ref pairs: {count}; use --summary full to describe them.")
    }
}

fn render_header(out: &mut String, fetch: &FetchInfo, language: Language) {
    match (&fetch.status, language) {
        (FetchStatus::Updated, Language::En) => {
            out.push_str(&format!("Fetched {}; factual ref graph:\n\n", fetch.remote));
        }
        (FetchStatus::Skipped, Language::En) => {
            out.push_str("Factual ref graph from local data:\n\n");
        }
        (FetchStatus::MissingRemote, Language::En) => {
            out.push_str(&format!(
                "Remote {} is not configured; factual ref graph from local data:\n\n",
                fetch.remote
            ));
        }
        (FetchStatus::Failed, Language::En) => {
            out.push_str(&format!(
                "Could not fetch {}; factual ref graph uses cached data:\n",
                fetch.remote
            ));
            if let Some(reason) = &fetch.reason {
                out.push_str(&format!("Reason: {reason}\n"));
            }
            out.push('\n');
        }
        (FetchStatus::Updated, Language::Ru) => {
            out.push_str(&format!(
                "Данные из {} получены; фактический граф refs:\n\n",
                fetch.remote
            ));
        }
        (FetchStatus::Skipped, Language::Ru) => {
            out.push_str("Фактический граф refs по локальным данным:\n\n");
        }
        (FetchStatus::MissingRemote, Language::Ru) => {
            out.push_str(&format!(
                "Удалённый репозиторий {} не настроен; фактический граф refs по локальным данным:\n\n",
                fetch.remote
            ));
        }
        (FetchStatus::Failed, Language::Ru) => {
            out.push_str(&format!(
                "Не удалось получить данные из {}; фактический граф refs построен по сохранённым данным:\n",
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
    edge: Option<(&str, u64)>,
    ancestors_last: &[bool],
    is_last: bool,
    ascii: bool,
    language: Language,
    visited: &mut HashSet<String>,
    lines: &mut Vec<TreeLine>,
) {
    let node = &graph.nodes[id];
    let repeated = !visited.insert(id.to_owned());
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
    let mut suffix = String::new();
    if node.current {
        suffix.push(' ');
        suffix.push_str(current_marker(language));
    }
    if repeated {
        suffix.push_str(if language.is_ru() {
            " (уже показана выше)"
        } else {
            " (already shown above)"
        });
    }
    let relation = edge
        .map(|(parent, ahead)| {
            Relation { ahead, behind: 0 }.compact(&graph.nodes[parent].name, language)
        })
        .unwrap_or_default();
    lines.push(TreeLine {
        prefix: left,
        label: node.display_name(),
        suffix,
        relation,
        current: node.current,
    });
    if repeated {
        return;
    }

    let children = graph.children(id);
    for (index, (child, child_edge)) in children.iter().enumerate() {
        let child_last = index + 1 == children.len();
        let mut next = ancestors_last.to_vec();
        next.push(child_last);
        collect_lines(
            graph,
            &child.id,
            Some((id, child_edge.ahead)),
            &next,
            child_last,
            ascii,
            language,
            visited,
            lines,
        );
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
    let mut synced: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for insight in insights {
        match insight {
            Insight::SyncedUpstream { branch, upstream } => {
                let remote = upstream
                    .split_once('/')
                    .map(|(name, _)| name)
                    .unwrap_or("remote");
                synced.entry(remote).or_default().push(branch);
            }
            Insight::UpstreamGone { branch, upstream } => lines.push(if language.is_ru() {
                format!("Локальная ветка {branch} отслеживает {upstream}, которой больше нет.")
            } else {
                format!("Local branch {branch} tracks {upstream}, which no longer exists.")
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
            Insight::Diverged {
                left,
                right,
                left_count,
                right_count,
                common_ancestors,
            } => {
                let base = if common_ancestors.is_empty() {
                    String::new()
                } else if language.is_ru() {
                    format!(
                        " Ближайшая общая отображаемая база: {}.",
                        common_ancestors.join(", ")
                    )
                } else {
                    format!(
                        " Nearest displayed common base: {}.",
                        common_ancestors.join(", ")
                    )
                };
                lines.push(if language.is_ru() {
                    format!(
                        "{left} и {right} разошлись: в {left} есть {}, а в {right} — {}.{base}",
                        unique_commits_ru(*left_count),
                        unique_commits_ru(*right_count)
                    )
                } else {
                    format!(
                        "{left} and {right} diverged: {left} has {}, while {right} has {}.{base}",
                        unique_commits_en(*left_count),
                        unique_commits_en(*right_count)
                    )
                });
            }
        }
    }
    for (remote, branches) in synced {
        lines.push(if language.is_ru() {
            if branches.len() == 1 {
                format!(
                    "{} совпадает с соответствующей веткой на {remote}.",
                    branches[0]
                )
            } else {
                format!(
                    "{} совпадают с соответствующими ветками на {remote}.",
                    join_names(&branches, language)
                )
            }
        } else if branches.len() == 1 {
            format!("{} matches its counterpart on {remote}.", branches[0])
        } else {
            format!(
                "{} match their counterparts on {remote}.",
                join_names(&branches, language)
            )
        });
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
