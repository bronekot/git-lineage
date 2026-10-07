use std::collections::HashSet;

use serde::Serialize;

use crate::{
    cli::SummaryMode,
    graph::BranchGraph,
    refs::{RefKind, RefSet},
    relation::RelationKind,
};

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Insight {
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
    SyncedUpstream {
        branch: String,
        upstream: String,
    },
    UpstreamGone {
        branch: String,
        upstream: String,
    },
    Diverged {
        left: String,
        right: String,
        left_count: u64,
        right_count: u64,
        common_ancestors: Vec<String>,
    },
}

#[derive(Debug, Clone, Default)]
pub struct Insights {
    pub items: Vec<Insight>,
    /// Diverged pairs left out by brief mode; `--summary full` lists them.
    pub hidden_diverged: usize,
}

pub fn build(refs: &RefSet, graph: &BranchGraph, mode: SummaryMode) -> Insights {
    if mode == SummaryMode::None {
        return Insights::default();
    }
    let mut result = Vec::new();
    let mut tracking_pairs = HashSet::new();

    for local in refs
        .refs
        .values()
        .filter(|item| item.kind == RefKind::Local)
    {
        let Some(upstream_name) = local.upstream.as_deref() else {
            continue;
        };
        let Some(upstream_ref) = refs.upstream_of(local) else {
            result.push(Insight::UpstreamGone {
                branch: local.name.clone(),
                upstream: short_ref_name(upstream_name).to_owned(),
            });
            continue;
        };
        let Some(local_id) = graph.node_id_for_oid(&local.oid) else {
            continue;
        };
        let Some(upstream_id) = graph.node_id_for_oid(&upstream_ref.oid) else {
            continue;
        };
        if local_id == upstream_id {
            result.push(Insight::SyncedUpstream {
                branch: local.name.clone(),
                upstream: upstream_ref.name.clone(),
            });
            continue;
        }
        tracking_pairs.insert(pair_key(local_id, upstream_id));
        let relation = graph
            .relation(local_id, upstream_id)
            .expect("different graph nodes have a pair relation");
        // `ahead` counts upstream-only commits, `behind` counts local-only ones.
        match relation.classify() {
            RelationKind::Synced => unreachable!("equal OIDs share one graph node"),
            RelationKind::Ahead(count) => result.push(Insight::TrackingBehind {
                branch: local.name.clone(),
                upstream: upstream_ref.name.clone(),
                count,
            }),
            RelationKind::Behind(count) => result.push(Insight::TrackingAhead {
                branch: local.name.clone(),
                upstream: upstream_ref.name.clone(),
                count,
            }),
            RelationKind::Diverged { behind, ahead } => result.push(Insight::TrackingDiverged {
                branch: local.name.clone(),
                upstream: upstream_ref.name.clone(),
                local_count: behind,
                remote_count: ahead,
            }),
        }
    }

    let current = graph.nodes.values().find(|node| node.current);
    let mut hidden_diverged = 0;
    for pair in &graph.pair_relations {
        if pair.left_only == 0 || pair.right_only == 0 {
            continue;
        }
        if tracking_pairs.contains(&pair_key(&pair.left, &pair.right)) {
            continue;
        }
        // Brief mode explains only how the current branch differs from its
        // siblings; every other divergence is already visible in the tree.
        if mode == SummaryMode::Brief {
            let involves_current =
                current.is_some_and(|node| node.id == pair.left || node.id == pair.right);
            if !involves_current || !graph.share_immediate_parent(&pair.left, &pair.right) {
                hidden_diverged += 1;
                continue;
            }
        }
        let common_ancestors = graph
            .nearest_common_ancestors(&pair.left, &pair.right)
            .into_iter()
            .map(|id| graph.nodes[&id].name.clone())
            .collect();
        result.push(Insight::Diverged {
            left: graph.nodes[&pair.left].name.clone(),
            right: graph.nodes[&pair.right].name.clone(),
            left_count: pair.left_only,
            right_count: pair.right_only,
            common_ancestors,
        });
    }
    Insights {
        items: result,
        hidden_diverged,
    }
}

fn short_ref_name(name: &str) -> &str {
    name.strip_prefix("refs/remotes/")
        .or_else(|| name.strip_prefix("refs/heads/"))
        .unwrap_or(name)
}

fn pair_key(left: &str, right: &str) -> (String, String) {
    if left <= right {
        (left.into(), right.into())
    } else {
        (right.into(), left.into())
    }
}
