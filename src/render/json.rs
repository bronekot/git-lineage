use serde::Serialize;

use crate::{
    error::Result, graph::BranchGraph, insights::Insight, refs::RefKind, render::FetchInfo,
};

#[derive(Serialize)]
struct JsonReport<'a> {
    repository: String,
    current_branch: Option<&'a str>,
    fetch: &'a FetchInfo,
    root: &'a str,
    nodes: Vec<JsonNode<'a>>,
    insights: &'a [Insight],
    warnings: &'a [String],
}

#[derive(Serialize)]
struct JsonNode<'a> {
    name: &'a str,
    kind: RefKind,
    parent: Option<&'a str>,
    ahead: u64,
    behind: u64,
    current: bool,
}

pub fn render(
    repository: &std::path::Path,
    current_branch: Option<&str>,
    graph: &BranchGraph,
    fetch: &FetchInfo,
    insights: &[Insight],
    warnings: &[String],
) -> Result<String> {
    let nodes = graph
        .nodes
        .values()
        .map(|node| JsonNode {
            name: &node.name,
            kind: node.kind,
            parent: node.parent.as_ref().map(|p| graph.nodes[p].name.as_str()),
            ahead: node.ahead.unwrap_or(0),
            behind: node.behind.unwrap_or(0),
            current: node.current,
        })
        .collect();
    let report = JsonReport {
        repository: repository.to_string_lossy().replace('\\', "/"),
        current_branch,
        fetch,
        root: &graph.nodes[&graph.root].name,
        nodes,
        insights,
        warnings,
    };
    Ok(serde_json::to_string_pretty(&report)?)
}
