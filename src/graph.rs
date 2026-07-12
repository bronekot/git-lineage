use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use serde::Serialize;

use crate::{
    config::LineageConfig,
    error::{ConfigIssue, Result},
    git::GitRepo,
    refs::{GitRef, RefKind, RefSet},
    relation::Relation,
};

#[derive(Debug, Clone, Serialize)]
pub struct GraphNode {
    #[serde(skip)]
    pub id: String,
    pub name: String,
    pub kind: RefKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ahead: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub behind: Option<u64>,
    pub current: bool,
    #[serde(skip)]
    pub tracking_remote: bool,
}

impl GraphNode {
    pub fn relation(&self) -> Option<Relation> {
        Some(Relation {
            ahead: self.ahead?,
            behind: self.behind?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct BranchGraph {
    pub root: String,
    pub nodes: BTreeMap<String, GraphNode>,
    pub unplaced: Vec<String>,
}

impl BranchGraph {
    #[allow(clippy::too_many_arguments)]
    pub fn build(
        repo: &GitRepo,
        refs: &RefSet,
        config: &LineageConfig,
        root_name: &str,
        remote: &str,
        all_remotes: bool,
        show_synced_remotes: bool,
        current_branch: Option<&str>,
    ) -> Result<Self> {
        let allowed = refs.filtered_ids(remote, all_remotes);
        let root_raw = refs.resolve(root_name)?;
        if !allowed.contains(&root_raw) {
            return Err(ConfigIssue::RootExcluded(root_name.into()).into());
        }

        let collapse = config.display.collapse_synced_upstreams && !show_synced_remotes;
        let mut aliases = HashMap::<String, String>::new();
        let mut upstream_pairs = Vec::<(String, String)>::new();

        for local in refs.refs.values().filter(|r| r.kind == RefKind::Local) {
            let Some(upstream_name) = local.upstream.as_deref() else {
                continue;
            };
            let Some(upstream_id) = refs.resolve_upstream(upstream_name) else {
                continue;
            };
            if !allowed.contains(&upstream_id) {
                continue;
            }
            upstream_pairs.push((local.full_name.clone(), upstream_id.clone()));
            let upstream = &refs.refs[&upstream_id];
            if collapse && local.oid == upstream.oid {
                aliases.insert(upstream_id.clone(), local.full_name.clone());
            }
        }

        let canonical = |id: String| aliases.get(&id).cloned().unwrap_or(id);
        let root = canonical(root_raw);

        let mut required = BTreeSet::new();
        for item in refs.refs.values().filter(|r| r.kind == RefKind::Local) {
            if allowed.contains(&item.full_name) {
                required.insert(item.full_name.clone());
            }
        }
        for (local, upstream) in &upstream_pairs {
            required.insert(local.clone());
            required.insert(upstream.clone());
        }
        required.insert(root.clone());

        // Explicit config references are always relevant, including remote-only refs.
        let mut configured_edges = Vec::new();
        for (child_name, parent_name) in &config.parents {
            let child = refs.resolve(child_name)?;
            let parent = refs.resolve(parent_name)?;
            if !allowed.contains(&child) || !allowed.contains(&parent) {
                return Err(ConfigIssue::EdgeOutOfScope {
                    child: child_name.clone(),
                    parent: parent_name.clone(),
                }
                .into());
            }
            required.insert(child.clone());
            required.insert(parent.clone());
            configured_edges.push((canonical(child), canonical(parent)));
        }
        if config.display.show_remote_only {
            required.extend(allowed.iter().cloned());
        }

        let current_id = current_branch.and_then(|name| refs.resolve(name).ok());
        let mut nodes = BTreeMap::new();
        for id in required {
            if aliases.contains_key(&id) {
                continue;
            }
            let item = refs
                .refs
                .get(&id)
                .ok_or_else(|| ConfigIssue::MissingRef(id.clone()))?;
            nodes.insert(
                id.clone(),
                node_from_ref(item, current_id.as_deref() == Some(&id)),
            );
        }

        // Configuration wins over inferred tracking relationships.
        for (child, parent) in configured_edges {
            if child == root {
                return Err(ConfigIssue::RootHasParent.into());
            }
            if child == parent {
                continue;
            }
            nodes
                .get_mut(&child)
                .ok_or_else(|| ConfigIssue::HiddenBranch(child.clone()))?
                .parent = Some(parent);
        }

        for (local_raw, upstream_raw) in upstream_pairs {
            let local = canonical(local_raw);
            let upstream = canonical(upstream_raw);
            if local == upstream || !nodes.contains_key(&upstream) {
                continue;
            }
            let upstream_node = nodes.get_mut(&upstream).expect("checked above");
            if upstream_node.parent.is_none() && upstream != root {
                upstream_node.parent = Some(local.clone());
            }
            if upstream_node.parent.as_deref() == Some(&local) {
                upstream_node.tracking_remote = true;
            }
        }

        if let Some(root_node) = nodes.get_mut(&root) {
            root_node.parent = None;
        }
        validate_parents(&nodes)?;

        let edges: Vec<_> = nodes
            .iter()
            .filter_map(|(id, node)| node.parent.as_ref().map(|p| (id.clone(), p.clone())))
            .collect();
        let pairs: Vec<_> = edges
            .iter()
            .map(|(child, parent)| (parent.clone(), child.clone()))
            .collect();
        let relations = Relation::between_many(repo, &pairs)?;
        for ((child, _parent), relation) in edges.into_iter().zip(relations) {
            let node = nodes.get_mut(&child).expect("edge came from nodes");
            node.ahead = Some(relation.ahead);
            node.behind = Some(relation.behind);
        }

        let mut unplaced: Vec<_> = nodes
            .values()
            .filter(|node| node.id != root && node.parent.is_none())
            .map(|node| node.name.clone())
            .collect();
        unplaced.sort();

        Ok(Self {
            root,
            nodes,
            unplaced,
        })
    }

    pub fn children(&self, parent: &str) -> Vec<&GraphNode> {
        let mut children: Vec<_> = self
            .nodes
            .values()
            .filter(|n| n.parent.as_deref() == Some(parent))
            .collect();
        children.sort_by(|a, b| sort_key(a).cmp(&sort_key(b)));
        children
    }

    pub fn reachable_ids(&self) -> BTreeSet<String> {
        let mut result = BTreeSet::new();
        let mut stack = vec![self.root.clone()];
        while let Some(id) = stack.pop() {
            if !result.insert(id.clone()) {
                continue;
            }
            stack.extend(self.children(&id).into_iter().map(|n| n.id.clone()));
        }
        result
    }

    pub fn leaf_ids(&self) -> Vec<String> {
        let reachable = self.reachable_ids();
        reachable
            .into_iter()
            .filter(|id| self.children(id).is_empty())
            .collect()
    }
}

fn node_from_ref(item: &GitRef, current: bool) -> GraphNode {
    GraphNode {
        id: item.full_name.clone(),
        name: item.name.clone(),
        kind: item.kind,
        parent: None,
        ahead: None,
        behind: None,
        current,
        tracking_remote: false,
    }
}

fn sort_key(node: &GraphNode) -> (u8, &str) {
    let group = if node.tracking_remote {
        0
    } else if node.kind == RefKind::Local {
        1
    } else {
        2
    };
    (group, node.name.as_str())
}

fn validate_parents(nodes: &BTreeMap<String, GraphNode>) -> Result<()> {
    for node in nodes.values() {
        if let Some(parent) = &node.parent {
            if !nodes.contains_key(parent) {
                return Err(ConfigIssue::MissingParent {
                    parent: parent.clone(),
                    child: node.name.clone(),
                }
                .into());
            }
        }
    }
    let parents: BTreeMap<_, _> = nodes
        .iter()
        .filter_map(|(id, n)| n.parent.as_ref().map(|p| (id.clone(), p.clone())))
        .collect();
    detect_cycles(&parents)
}

fn detect_cycles(parents: &BTreeMap<String, String>) -> Result<()> {
    for start in parents.keys() {
        let mut seen = HashSet::new();
        let mut cursor = start.as_str();
        while let Some(parent) = parents.get(cursor) {
            if !seen.insert(cursor.to_owned()) {
                return Err(ConfigIssue::Cycle(cursor.into()).into());
            }
            cursor = parent;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_node(id: &str, name: &str, kind: RefKind, tracking_remote: bool) -> GraphNode {
        GraphNode {
            id: id.into(),
            name: name.into(),
            kind,
            parent: (id != "root").then(|| "root".into()),
            ahead: None,
            behind: None,
            current: false,
            tracking_remote,
        }
    }

    #[test]
    fn detects_cycle() {
        let parents = BTreeMap::from([("a".into(), "b".into()), ("b".into(), "a".into())]);
        assert!(detect_cycles(&parents).is_err());
    }

    #[test]
    fn accepts_acyclic_map() {
        let parents = BTreeMap::from([("b".into(), "a".into()), ("c".into(), "b".into())]);
        assert!(detect_cycles(&parents).is_ok());
    }

    #[test]
    fn sorts_tracking_remote_then_local_then_other_remote() {
        let nodes = BTreeMap::from([
            (
                "root".into(),
                test_node("root", "main", RefKind::Local, false),
            ),
            (
                "remote-other".into(),
                test_node("remote-other", "upstream/z", RefKind::Remote, false),
            ),
            (
                "local".into(),
                test_node("local", "alpha", RefKind::Local, false),
            ),
            (
                "remote-tracking".into(),
                test_node("remote-tracking", "origin/main", RefKind::Remote, true),
            ),
        ]);
        let graph = BranchGraph {
            root: "root".into(),
            nodes,
            unplaced: Vec::new(),
        };
        let names: Vec<_> = graph
            .children("root")
            .into_iter()
            .map(|node| node.name.as_str())
            .collect();
        assert_eq!(names, ["origin/main", "alpha", "upstream/z"]);
    }
}
