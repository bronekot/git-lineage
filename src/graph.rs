use std::collections::{BTreeMap, HashMap};

use serde::Serialize;

use crate::{
    error::{LineageError, Result},
    git::GitRepo,
    refs::{GitRef, RefKind, RefSet},
    relation::Relation,
};

#[derive(Debug, Clone, Serialize)]
pub struct GraphParent {
    #[serde(skip)]
    pub id: String,
    pub ahead: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphNode {
    #[serde(skip)]
    pub id: String,
    #[serde(skip)]
    pub oid: String,
    pub name: String,
    pub aliases: Vec<String>,
    pub kind: RefKind,
    pub parents: Vec<GraphParent>,
    pub current: bool,
}

impl GraphNode {
    pub fn display_name(&self) -> String {
        if self.aliases.is_empty() {
            self.name.clone()
        } else {
            format!("{} (= {})", self.name, self.aliases.join(", "))
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PairRelation {
    #[serde(skip)]
    pub left: String,
    #[serde(skip)]
    pub right: String,
    pub left_only: u64,
    pub right_only: u64,
}

/// Commit counts for every node history and for every pairwise intersection.
#[derive(Debug, Clone)]
struct HistoryMatrix {
    len: usize,
    sizes: Vec<u64>,
    shared: Vec<u64>,
}

impl HistoryMatrix {
    fn relation(&self, left: usize, right: usize) -> Relation {
        let both = self.shared[left * self.len + right];
        Relation {
            behind: self.sizes[left] - both,
            ahead: self.sizes[right] - both,
        }
    }

    fn is_strict_ancestor(&self, ancestor: usize, descendant: usize) -> bool {
        let relation = self.relation(ancestor, descendant);
        relation.behind == 0 && relation.ahead > 0
    }
}

#[derive(Debug, Clone)]
pub struct BranchGraph {
    pub roots: Vec<String>,
    pub nodes: BTreeMap<String, GraphNode>,
    pub pair_relations: Vec<PairRelation>,
    ids: Vec<String>,
    index: HashMap<String, usize>,
    history: HistoryMatrix,
    pub total_refs: usize,
    pub merged_aliases: usize,
    pub edge_count: usize,
}

impl BranchGraph {
    /// Builds the graph; `current_branch` is a full ref name such as `refs/heads/main`.
    pub fn build(repo: &GitRepo, refs: &RefSet, current_branch: Option<&str>) -> Result<Self> {
        if refs.refs.is_empty() {
            return Err(LineageError::NoRefs);
        }

        let current_id = current_branch.filter(|name| refs.refs.contains_key(*name));
        let mut by_oid = BTreeMap::<String, Vec<&GitRef>>::new();
        for item in refs.refs.values() {
            by_oid.entry(item.oid.clone()).or_default().push(item);
        }

        let total_refs = refs.refs.len();
        let merged_aliases = total_refs.saturating_sub(by_oid.len());

        let mut nodes = BTreeMap::new();
        for (oid, mut group) in by_oid {
            group.sort_by_key(|item| (primary_rank(item, current_id), item.name.as_str()));
            let primary = group[0];
            let id = primary.full_name.clone();
            let mut aliases: Vec<_> = group[1..].iter().map(|item| item.name.clone()).collect();
            aliases.sort();
            let current = group
                .iter()
                .any(|item| current_id == Some(item.full_name.as_str()));
            nodes.insert(
                id.clone(),
                GraphNode {
                    id,
                    oid,
                    name: primary.name.clone(),
                    aliases,
                    kind: primary.kind,
                    parents: Vec::new(),
                    current,
                },
            );
        }

        let ids: Vec<_> = nodes.keys().cloned().collect();
        let index: HashMap<_, _> = ids
            .iter()
            .enumerate()
            .map(|(position, id)| (id.clone(), position))
            .collect();
        let tips: Vec<_> = ids.iter().map(|id| nodes[id].oid.as_str()).collect();
        let history = read_history_matrix(repo, &tips)?;

        let mut pair_relations = Vec::new();
        for left in 0..ids.len() {
            for right in (left + 1)..ids.len() {
                let relation = history.relation(left, right);
                pair_relations.push(PairRelation {
                    left: ids[left].clone(),
                    right: ids[right].clone(),
                    left_only: relation.behind,
                    right_only: relation.ahead,
                });
            }
        }

        let edges = transitive_reduction(ids.len(), |ancestor, descendant| {
            history.is_strict_ancestor(ancestor, descendant)
        });
        for &(parent, child) in &edges {
            let ahead = history.relation(parent, child).ahead;
            nodes
                .get_mut(&ids[child])
                .expect("reduction used an indexed node")
                .parents
                .push(GraphParent {
                    id: ids[parent].clone(),
                    ahead,
                });
        }
        let names: HashMap<_, _> = nodes
            .iter()
            .map(|(id, node)| (id.clone(), node.name.clone()))
            .collect();
        for node in nodes.values_mut() {
            node.parents.sort_by(|a, b| names[&a.id].cmp(&names[&b.id]));
        }

        let mut roots: Vec<_> = nodes
            .values()
            .filter(|node| node.parents.is_empty())
            .map(|node| node.id.clone())
            .collect();
        roots.sort_by(|a, b| root_sort_key(&nodes[a]).cmp(&root_sort_key(&nodes[b])));

        Ok(Self {
            roots,
            nodes,
            pair_relations,
            ids,
            index,
            history,
            total_refs,
            merged_aliases,
            edge_count: edges.len(),
        })
    }

    pub fn children(&self, parent: &str) -> Vec<(&GraphNode, &GraphParent)> {
        let mut children: Vec<_> = self
            .nodes
            .values()
            .filter_map(|node| {
                node.parents
                    .iter()
                    .find(|edge| edge.id == parent)
                    .map(|edge| (node, edge))
            })
            .collect();
        children.sort_by_key(|(node, _)| node_sort_key(node));
        children
    }

    pub fn relation(&self, left: &str, right: &str) -> Option<Relation> {
        let left = *self.index.get(left)?;
        let right = *self.index.get(right)?;
        Some(self.history.relation(left, right))
    }

    pub fn node_id_for_oid(&self, oid: &str) -> Option<&str> {
        self.nodes
            .values()
            .find(|node| node.oid == oid)
            .map(|node| node.id.as_str())
    }

    pub fn share_immediate_parent(&self, left: &str, right: &str) -> bool {
        self.nodes[left].parents.iter().any(|left_parent| {
            self.nodes[right]
                .parents
                .iter()
                .any(|right_parent| right_parent.id == left_parent.id)
        })
    }

    pub fn nearest_common_ancestors(&self, left: &str, right: &str) -> Vec<String> {
        let (Some(&left), Some(&right)) = (self.index.get(left), self.index.get(right)) else {
            return Vec::new();
        };
        let common: Vec<_> = (0..self.ids.len())
            .filter(|&candidate| {
                self.history.is_strict_ancestor(candidate, left)
                    && self.history.is_strict_ancestor(candidate, right)
            })
            .collect();
        let mut nearest: Vec<_> = common
            .iter()
            .filter(|&&candidate| {
                !common
                    .iter()
                    .any(|&other| self.history.is_strict_ancestor(candidate, other))
            })
            .map(|&candidate| self.ids[candidate].clone())
            .collect();
        nearest.sort_by(|a, b| self.nodes[a].name.cmp(&self.nodes[b].name));
        nearest
    }
}

fn primary_rank(item: &GitRef, current: Option<&str>) -> u8 {
    if current == Some(item.full_name.as_str()) {
        0
    } else if item.kind == RefKind::Local {
        1
    } else {
        2
    }
}

/// Counts the commits reachable from each tip and from each pair of tips.
///
/// Every commit receives a bitset of the tips that reach it, propagated from
/// children to parents in a single topological pass. Commits with the same
/// bitset are counted together, so the pair counts cost one pass over the
/// distinct bitsets instead of one set difference per pair.
fn read_history_matrix(repo: &GitRepo, tips: &[&str]) -> Result<HistoryMatrix> {
    let len = tips.len();
    let mut input = tips.join("\n");
    input.push('\n');
    let bytes = repo.checked_with_input(
        ["rev-list", "--topo-order", "--parents", "--stdin"],
        input.into_bytes(),
    )?;
    let text = String::from_utf8_lossy(&bytes);

    let mut commit_index = HashMap::<&str, usize>::new();
    let mut parents = Vec::<Vec<usize>>::new();
    let mut order = Vec::new();
    let mut intern = |oid, parents: &mut Vec<Vec<usize>>| {
        *commit_index.entry(oid).or_insert_with(|| {
            parents.push(Vec::new());
            parents.len() - 1
        })
    };
    for line in text.lines() {
        let mut fields = line.split_whitespace();
        let Some(oid) = fields.next() else { continue };
        let commit = intern(oid, &mut parents);
        let commit_parents: Vec<_> = fields.map(|oid| intern(oid, &mut parents)).collect();
        parents[commit] = commit_parents;
        order.push(commit);
    }

    let words = len.div_ceil(64).max(1);
    let mut reach = vec![0u64; parents.len() * words];
    for (tip_index, tip) in tips.iter().enumerate() {
        if let Some(&commit) = commit_index.get(tip) {
            reach[commit * words + tip_index / 64] |= 1 << (tip_index % 64);
        }
    }
    // `--topo-order` lists every child before its parents.
    let mut row = vec![0u64; words];
    for &commit in &order {
        row.copy_from_slice(&reach[commit * words..(commit + 1) * words]);
        for &parent in &parents[commit] {
            for (target, bits) in reach[parent * words..(parent + 1) * words]
                .iter_mut()
                .zip(&row)
            {
                *target |= bits;
            }
        }
    }

    let mut groups = HashMap::<&[u64], u64>::new();
    for &commit in &order {
        *groups
            .entry(&reach[commit * words..(commit + 1) * words])
            .or_default() += 1;
    }
    let mut sizes = vec![0u64; len];
    let mut shared = vec![0u64; len * len];
    for (bits, count) in groups {
        let members: Vec<_> = (0..len)
            .filter(|&tip| bits[tip / 64] & (1 << (tip % 64)) != 0)
            .collect();
        for &left in &members {
            sizes[left] += count;
            for &right in &members {
                shared[left * len + right] += count;
            }
        }
    }
    Ok(HistoryMatrix { len, sizes, shared })
}

/// Returns `(parent, child)` edges of the strict-inclusion order with every
/// edge implied by a longer path removed.
fn transitive_reduction(
    len: usize,
    is_strict_ancestor: impl Fn(usize, usize) -> bool,
) -> Vec<(usize, usize)> {
    let mut edges = Vec::new();
    for child in 0..len {
        let ancestors: Vec<_> = (0..len)
            .filter(|&candidate| is_strict_ancestor(candidate, child))
            .collect();
        for &parent in &ancestors {
            let transitive = ancestors
                .iter()
                .any(|&middle| middle != parent && is_strict_ancestor(parent, middle));
            if !transitive {
                edges.push((parent, child));
            }
        }
    }
    edges.sort_unstable();
    edges
}

fn node_sort_key(node: &GraphNode) -> (u8, &str) {
    let kind = if node.kind == RefKind::Local { 0 } else { 1 };
    (kind, node.name.as_str())
}

fn root_sort_key(node: &GraphNode) -> (u8, u8, &str) {
    let conventional = match node.name.as_str() {
        "main" | "origin/main" => 0,
        "master" | "origin/master" => 1,
        _ => 2,
    };
    let kind = if node.kind == RefKind::Local { 0 } else { 1 };
    (conventional, kind, node.name.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a matrix from explicit history sets given as commit lists.
    fn matrix(histories: &[&[u32]]) -> HistoryMatrix {
        let len = histories.len();
        let mut shared = vec![0; len * len];
        for left in 0..len {
            for right in 0..len {
                shared[left * len + right] = histories[left]
                    .iter()
                    .filter(|commit| histories[right].contains(commit))
                    .count() as u64;
            }
        }
        HistoryMatrix {
            len,
            sizes: histories.iter().map(|h| h.len() as u64).collect(),
            shared,
        }
    }

    #[test]
    fn matrix_reports_left_and_right_only_counts() {
        let history = matrix(&[&[1, 2, 3], &[1, 2, 4, 5]]);
        assert_eq!(
            history.relation(0, 1),
            Relation {
                behind: 1,
                ahead: 2
            }
        );
        assert!(!history.is_strict_ancestor(0, 1));
        assert!(!history.is_strict_ancestor(0, 0));
    }

    #[test]
    fn reduction_removes_transitive_inclusion_edge() {
        // main ⊂ middle ⊂ tip
        let history = matrix(&[&[1], &[1, 2], &[1, 2, 3]]);
        assert_eq!(
            transitive_reduction(3, |a, d| history.is_strict_ancestor(a, d)),
            [(0, 1), (1, 2)]
        );
    }

    #[test]
    fn reduction_keeps_both_merge_parents() {
        // main, left, right, merge
        let history = matrix(&[&[1], &[1, 2], &[1, 3], &[1, 2, 3, 4]]);
        let edges = transitive_reduction(4, |a, d| history.is_strict_ancestor(a, d));
        assert_eq!(edges, [(0, 1), (0, 2), (1, 3), (2, 3)]);
    }

    #[test]
    fn display_name_keeps_every_alias() {
        let node = GraphNode {
            id: "refs/heads/dev".into(),
            oid: "a".into(),
            name: "dev".into(),
            aliases: vec!["origin/dev".into(), "origin/player-fix".into()],
            kind: RefKind::Local,
            parents: Vec::new(),
            current: false,
        };
        assert_eq!(node.display_name(), "dev (= origin/dev, origin/player-fix)");
    }
}
