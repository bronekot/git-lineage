use serde::Serialize;

use crate::{
    error::Result,
    graph::BranchGraph,
    insights::{Insight, Insights},
    refs::RefKind,
    render::FetchInfo,
};

#[derive(Serialize)]
struct JsonReport<'a> {
    repository: String,
    current_branch: Option<&'a str>,
    fetch: &'a FetchInfo,
    roots: Vec<&'a str>,
    refs: RefStats,
    nodes: Vec<JsonNode<'a>>,
    relations: Vec<JsonRelation<'a>>,
    insights: &'a [Insight],
    hidden_diverged_pairs: usize,
    warnings: &'a [String],
}

#[derive(Serialize)]
struct JsonNode<'a> {
    name: &'a str,
    aliases: &'a [String],
    kind: RefKind,
    parents: Vec<JsonParent<'a>>,
    current: bool,
}

#[derive(Serialize)]
struct JsonParent<'a> {
    name: &'a str,
    ahead: u64,
}

#[derive(Serialize)]
struct JsonRelation<'a> {
    left: &'a str,
    right: &'a str,
    left_only: u64,
    right_only: u64,
    relation: &'static str,
}

#[derive(Serialize)]
struct RefStats {
    discovered: usize,
    nodes: usize,
    merged_aliases: usize,
    reduced_edges: usize,
}

pub fn render(
    repository: &std::path::Path,
    current_branch: Option<&str>,
    graph: &BranchGraph,
    fetch: &FetchInfo,
    insights: &Insights,
    warnings: &[String],
) -> Result<String> {
    let nodes = graph
        .nodes
        .values()
        .map(|node| JsonNode {
            name: &node.name,
            aliases: &node.aliases,
            kind: node.kind,
            parents: node
                .parents
                .iter()
                .map(|parent| JsonParent {
                    name: &graph.nodes[&parent.id].name,
                    ahead: parent.ahead,
                })
                .collect(),
            current: node.current,
        })
        .collect();
    let relations = graph
        .pair_relations
        .iter()
        .map(|pair| JsonRelation {
            left: &graph.nodes[&pair.left].name,
            right: &graph.nodes[&pair.right].name,
            left_only: pair.left_only,
            right_only: pair.right_only,
            // Equal commits share one node, so a pair always differs.
            relation: match (pair.left_only, pair.right_only) {
                (0, _) => "left_ancestor",
                (_, 0) => "right_ancestor",
                _ => "diverged",
            },
        })
        .collect();
    let report = JsonReport {
        repository: repository.to_string_lossy().replace('\\', "/"),
        current_branch,
        fetch,
        roots: graph
            .roots
            .iter()
            .map(|id| graph.nodes[id].name.as_str())
            .collect(),
        refs: RefStats {
            discovered: graph.total_refs,
            nodes: graph.nodes.len(),
            merged_aliases: graph.merged_aliases,
            reduced_edges: graph.edge_count,
        },
        nodes,
        relations,
        insights: &insights.items,
        hidden_diverged_pairs: insights.hidden_diverged,
        warnings,
    };
    Ok(serde_json::to_string_pretty(&report)?)
}
