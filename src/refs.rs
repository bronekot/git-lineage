use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::{
    error::{ConfigIssue, Result},
    git::GitRepo,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RefKind {
    Local,
    Remote,
}

#[derive(Debug, Clone, Serialize)]
pub struct GitRef {
    pub full_name: String,
    pub name: String,
    pub oid: String,
    pub kind: RefKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RefSet {
    pub refs: BTreeMap<String, GitRef>,
}

impl RefSet {
    pub fn read(repo: &GitRepo) -> Result<Self> {
        let bytes = repo.checked([
            "for-each-ref",
            "--format=%(refname)%00%(refname:short)%00%(objectname)%00%(upstream)",
            "refs/heads",
            "refs/remotes",
        ])?;
        let text = String::from_utf8_lossy(&bytes);
        let mut refs = BTreeMap::new();
        for line in text.lines() {
            let fields: Vec<_> = line.split('\0').collect();
            if fields.len() < 4 || fields[0].is_empty() {
                continue;
            }
            let kind = if fields[0].starts_with("refs/heads/") {
                RefKind::Local
            } else if fields[0].starts_with("refs/remotes/") {
                RefKind::Remote
            } else {
                continue;
            };
            let name = match kind {
                RefKind::Local => fields[0].trim_start_matches("refs/heads/"),
                RefKind::Remote => fields[0].trim_start_matches("refs/remotes/"),
            };
            // Ignore symbolic remote HEAD aliases such as origin/HEAD.
            if kind == RefKind::Remote && name.ends_with("/HEAD") {
                continue;
            }
            let upstream = (!fields[3].is_empty()).then(|| fields[3].to_owned());
            let item = GitRef {
                full_name: fields[0].to_owned(),
                name: name.to_owned(),
                oid: fields[2].to_owned(),
                kind,
                upstream,
            };
            refs.insert(item.full_name.clone(), item);
        }
        Ok(Self { refs })
    }

    pub fn resolve(&self, name: &str) -> Result<String> {
        if let Some(item) = self.refs.get(name) {
            return Ok(item.full_name.clone());
        }
        let matches: Vec<_> = self.refs.values().filter(|r| r.name == name).collect();
        match matches.as_slice() {
            [item] => Ok(item.full_name.clone()),
            [] => Err(ConfigIssue::RefNotFound(name.into()).into()),
            _ => Err(ConfigIssue::AmbiguousRef(name.into()).into()),
        }
    }

    pub fn resolve_upstream(&self, upstream: &str) -> Option<String> {
        if self.refs.contains_key(upstream) {
            Some(upstream.to_owned())
        } else {
            self.refs
                .values()
                .find(|r| r.name == upstream)
                .map(|r| r.full_name.clone())
        }
    }

    pub fn filtered_ids(&self, remote: &str, all_remotes: bool) -> BTreeSet<String> {
        self.refs
            .values()
            .filter(|r| match r.kind {
                RefKind::Local => true,
                RefKind::Remote if all_remotes => true,
                RefKind::Remote => r.name.starts_with(&format!("{remote}/")),
            })
            .map(|r| r.full_name.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(full: &str, short: &str) -> GitRef {
        GitRef {
            full_name: full.into(),
            name: short.into(),
            oid: "a".into(),
            kind: RefKind::Local,
            upstream: None,
        }
    }

    #[test]
    fn resolves_short_and_full_names() {
        let mut refs = BTreeMap::new();
        refs.insert("refs/heads/main".into(), item("refs/heads/main", "main"));
        let set = RefSet { refs };
        assert_eq!(set.resolve("main").unwrap(), "refs/heads/main");
        assert_eq!(set.resolve("refs/heads/main").unwrap(), "refs/heads/main");
    }

    #[test]
    fn rejects_ambiguous_short_name() {
        let mut refs = BTreeMap::new();
        refs.insert("refs/heads/x".into(), item("refs/heads/x", "same"));
        refs.insert("refs/heads/y".into(), item("refs/heads/y", "same"));
        let set = RefSet { refs };
        assert!(set
            .resolve("same")
            .unwrap_err()
            .to_string()
            .contains("ambiguous"));
    }
}
