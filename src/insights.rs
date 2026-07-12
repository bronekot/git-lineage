use std::collections::BTreeMap;

use serde::Serialize;

use crate::{
    cli::SummaryMode,
    error::Result,
    git::GitRepo,
    graph::BranchGraph,
    refs::{RefKind, RefSet},
    relation::{Relation, RelationKind},
};

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Insight {
    RootBehind {
        root: String,
        branch: String,
        count: u64,
    },
    TrackingBehind {
        branch: String,
        upstream: String,
        count: u64,
    },
    TrackingAhead {
        branch: String,
        upstream: String,
        count: u64,
    },
    TrackingDiverged {
        branch: String,
        upstream: String,
        local_count: u64,
        remote_count: u64,
    },
    Siblings {
        parent: String,
        left: String,
        right: String,
        left_count: u64,
        right_count: u64,
    },
    SyncedUpstream {
        branch: String,
        upstream: String,
    },
    AutoParent {
        branch: String,
        parent: String,
        distance: u64,
    },
}

pub fn build(
    repo: &GitRepo,
    refs: &RefSet,
    graph: &BranchGraph,
    mode: SummaryMode,
) -> Result<Vec<Insight>> {
    if mode == SummaryMode::None {
        return Ok(Vec::new());
    }
    let mut result = Vec::new();
    let root_name = graph.nodes[&graph.root].name.clone();

    for leaf in graph.leaf_ids() {
        if leaf == graph.root {
            continue;
        }
        let relation = if graph.nodes[&leaf].parent.as_deref() == Some(&graph.root) {
            graph.nodes[&leaf]
                .relation()
                .expect("graph edges always have a relation")
        } else {
            Relation::between(repo, &graph.root, &leaf)?
        };
        if relation.behind == 0 && relation.ahead > 0 {
            result.push(Insight::RootBehind {
                root: root_name.clone(),
                branch: graph.nodes[&leaf].name.clone(),
                count: relation.ahead,
            });
        }
    }

    for local in refs.refs.values().filter(|r| r.kind == RefKind::Local) {
        let Some(upstream_name) = local.upstream.as_deref() else {
            continue;
        };
        let Some(upstream_id) = refs.resolve_upstream(upstream_name) else {
            continue;
        };
        let upstream = &refs.refs[&upstream_id];
        let relation = if local.oid == upstream.oid {
            Relation {
                ahead: 0,
                behind: 0,
            }
        } else if let Some(node) = graph.nodes.get(&upstream.full_name) {
            if node.parent.as_deref() == Some(&local.full_name) {
                node.relation().expect("graph edges always have a relation")
            } else {
                Relation::between(repo, &local.full_name, &upstream.full_name)?
            }
        } else {
            Relation::between(repo, &local.full_name, &upstream.full_name)?
        };
        match relation.classify() {
            RelationKind::Synced => result.push(Insight::SyncedUpstream {
                branch: local.name.clone(),
                upstream: upstream.name.clone(),
            }),
            RelationKind::Ahead(count) => result.push(Insight::TrackingBehind {
                branch: local.name.clone(),
                upstream: upstream.name.clone(),
                count,
            }),
            RelationKind::Behind(count) => result.push(Insight::TrackingAhead {
                branch: local.name.clone(),
                upstream: upstream.name.clone(),
                count,
            }),
            RelationKind::Diverged { behind, ahead } => result.push(Insight::TrackingDiverged {
                branch: local.name.clone(),
                upstream: upstream.name.clone(),
                local_count: behind,
                remote_count: ahead,
            }),
        }
    }

    let reachable = graph.reachable_ids();
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for id in &reachable {
        let node = &graph.nodes[id];
        let Some(parent) = &node.parent else { continue };
        if mode == SummaryMode::Brief && !graph.children(id).is_empty() {
            continue;
        }
        groups.entry(parent.clone()).or_default().push(id.clone());
    }
    let mut sibling_pairs = Vec::new();
    for (parent, mut children) in groups {
        children.sort_by_key(|id| graph.nodes[id].name.clone());
        if mode == SummaryMode::Brief {
            for pair in children.windows(2) {
                sibling_pairs.push((parent.clone(), pair[0].clone(), pair[1].clone()));
            }
        } else {
            for left_index in 0..children.len() {
                for right_index in (left_index + 1)..children.len() {
                    sibling_pairs.push((
                        parent.clone(),
                        children[left_index].clone(),
                        children[right_index].clone(),
                    ));
                }
            }
        }
    }
    let relation_pairs: Vec<_> = sibling_pairs
        .iter()
        .map(|(_, left, right)| (left.clone(), right.clone()))
        .collect();
    let relations = Relation::between_many(repo, &relation_pairs)?;
    for ((parent, left, right), relation) in sibling_pairs.into_iter().zip(relations) {
        if relation.ahead == 0 && relation.behind == 0 {
            continue;
        }
        result.push(Insight::Siblings {
            parent: graph.nodes[&parent].name.clone(),
            left: graph.nodes[&left].name.clone(),
            right: graph.nodes[&right].name.clone(),
            left_count: relation.behind,
            right_count: relation.ahead,
        });
    }
    Ok(result)
}
