use std::collections::{BTreeMap, HashMap, HashSet};

use crate::{
    config::LineageConfig,
    error::Result,
    git::GitRepo,
    refs::{GitRef, RefKind, RefSet},
    relation::{is_ancestor, Relation},
};

#[derive(Debug)]
pub struct Discovery {
    pub config: LineageConfig,
    pub ambiguous: Vec<AmbiguousParent>,
    pub unplaced: Vec<String>,
    pub chosen: Vec<DiscoveredParent>,
}

#[derive(Debug, Clone)]
pub struct AmbiguousParent {
    pub branch: String,
    pub candidates: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct DiscoveredParent {
    pub child_id: String,
    pub child: String,
    pub parent: String,
    pub distance: u64,
}

pub fn discover(
    repo: &GitRepo,
    refs: &RefSet,
    mut config: LineageConfig,
    root_name: &str,
    remote: &str,
    all_remotes: bool,
) -> Result<Discovery> {
    let root = refs.resolve(root_name)?;
    let allowed = refs.filtered_ids(remote, all_remotes);
    let candidates: Vec<_> = allowed.iter().filter_map(|id| refs.refs.get(id)).collect();
    let mut aliases = HashMap::new();
    let mut tracking_edges = Vec::new();
    for local in refs
        .refs
        .values()
        .filter(|item| item.kind == RefKind::Local)
    {
        let Some(upstream) = local
            .upstream
            .as_deref()
            .and_then(|name| refs.resolve_upstream(name))
        else {
            continue;
        };
        if !allowed.contains(&upstream) {
            continue;
        }
        if refs.refs[&upstream].oid == local.oid {
            aliases.insert(upstream, local.full_name.clone());
        } else {
            tracking_edges.push((upstream, local.full_name.clone()));
        }
    }
    let canonical = |id: String| aliases.get(&id).cloned().unwrap_or(id);
    let mut graph_parents = HashMap::new();
    for (child, parent) in tracking_edges {
        let child = canonical(child);
        let parent = canonical(parent);
        if child != parent {
            graph_parents.insert(child, parent);
        }
    }
    for (child, parent) in &config.parents {
        let child = canonical(refs.resolve(child)?);
        let parent = canonical(refs.resolve(parent)?);
        if child != parent {
            graph_parents.insert(child, parent);
        }
    }
    let mut parents = config.parents.clone();
    let configured_children: Vec<_> = config
        .parents
        .keys()
        .map(|name| refs.resolve(name))
        .collect::<Result<_>>()?;
    let mut ambiguous = Vec::new();
    let mut unplaced = Vec::new();
    let mut chosen = Vec::new();

    for child in candidates
        .iter()
        .copied()
        .filter(|r| r.kind == RefKind::Local && r.full_name != root)
    {
        if configured_children.contains(&child.full_name) {
            continue;
        }
        let tracking_upstream = child
            .upstream
            .as_deref()
            .and_then(|upstream| refs.resolve_upstream(upstream));
        let child_id = canonical(child.full_name.clone());
        let mut matches = BTreeMap::<(u64, u8), Vec<&GitRef>>::new();
        for candidate in &candidates {
            // The tracking relationship is represented separately in the graph.
            // Treating a branch's own upstream as its logical parent would add
            // the opposite tracking edge as well and create a two-node cycle.
            if candidate.full_name == child.full_name
                || candidate.oid == child.oid
                || tracking_upstream.as_deref() == Some(&candidate.full_name)
            {
                continue;
            }
            if !is_ancestor(repo, &candidate.full_name, &child.full_name)? {
                continue;
            }
            let distance = Relation::between(repo, &candidate.full_name, &child.full_name)?.ahead;
            matches
                .entry((distance, priority(candidate, remote)))
                .or_default()
                .push(candidate);
        }

        let mut selected = None;
        for ((distance, _priority), group) in matches {
            let viable: Vec<_> = group
                .into_iter()
                .filter(|candidate| {
                    let parent_id = canonical(candidate.full_name.clone());
                    !would_create_cycle(&graph_parents, &child_id, &parent_id)
                })
                .collect();
            match viable.as_slice() {
                [] => continue,
                [parent] => {
                    selected = Some((*parent, distance));
                    break;
                }
                many => {
                    let candidates = many
                        .iter()
                        .map(|item| item.name.clone())
                        .collect::<Vec<_>>();
                    ambiguous.push(AmbiguousParent {
                        branch: child.name.clone(),
                        candidates,
                    });
                    selected = None;
                    break;
                }
            }
        }

        let Some((parent, best_distance)) = selected else {
            if !ambiguous.iter().any(|item| item.branch == child.name) {
                unplaced.push(child.name.clone());
            }
            continue;
        };
        let parent_id = canonical(parent.full_name.clone());
        graph_parents.insert(child_id, parent_id);
        parents.insert(config_name(refs, child), config_name(refs, parent));
        chosen.push(DiscoveredParent {
            child_id: child.full_name.clone(),
            child: child.name.clone(),
            parent: parent.name.clone(),
            distance: best_distance,
        });
    }

    config.root = root_name.to_owned();
    config.remote = remote.to_owned();
    config.parents = parents;
    Ok(Discovery {
        config,
        ambiguous,
        unplaced,
        chosen,
    })
}

fn would_create_cycle(parents: &HashMap<String, String>, child: &str, parent: &str) -> bool {
    if child == parent {
        return true;
    }
    let mut cursor = parent;
    let mut seen = HashSet::new();
    while let Some(next) = parents.get(cursor) {
        if next == child {
            return true;
        }
        if !seen.insert(cursor) {
            return true;
        }
        cursor = next;
    }
    false
}

fn config_name(refs: &RefSet, item: &GitRef) -> String {
    match refs.resolve(&item.name) {
        Ok(id) if id == item.full_name => item.name.clone(),
        _ => item.full_name.clone(),
    }
}

fn priority(candidate: &GitRef, remote: &str) -> u8 {
    if candidate.kind == RefKind::Remote && candidate.name.starts_with(&format!("{remote}/")) {
        0
    } else if candidate.kind == RefKind::Local {
        1
    } else {
        2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_logical_parent_that_closes_a_tracking_cycle() {
        let parents = HashMap::from([
            ("origin/dev".into(), "dev".into()),
            ("origin/commands".into(), "commands".into()),
            ("commands".into(), "origin/dev".into()),
        ]);

        assert!(would_create_cycle(&parents, "dev", "origin/commands"));
        assert!(!would_create_cycle(&parents, "dev", "main"));
    }
}
