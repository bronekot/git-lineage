use std::collections::BTreeMap;

use serde::Serialize;

use crate::{error::Result, git::GitRepo};

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
            "--format=%(refname)%00%(objectname)%00%(*objectname)%00%(upstream)%00%(symref)",
            "refs/heads",
            "refs/remotes",
        ])?;
        let text = String::from_utf8_lossy(&bytes);
        let mut refs = BTreeMap::new();
        for line in text.lines() {
            let fields: Vec<_> = line.split('\0').collect();
            let [full_name, oid, peeled, upstream, symref] = fields[..] else {
                continue;
            };
            // Symbolic refs such as origin/HEAD are aliases, not branches.
            if !symref.is_empty() {
                continue;
            }
            let (kind, name) = if let Some(name) = full_name.strip_prefix("refs/heads/") {
                (RefKind::Local, name)
            } else if let Some(name) = full_name.strip_prefix("refs/remotes/") {
                (RefKind::Remote, name)
            } else {
                continue;
            };
            let upstream = (!upstream.is_empty()).then(|| upstream.to_owned());
            let item = GitRef {
                full_name: full_name.to_owned(),
                name: name.to_owned(),
                // A branch may point at an annotated tag; use the commit it tags.
                oid: if peeled.is_empty() { oid } else { peeled }.to_owned(),
                kind,
                upstream,
            };
            refs.insert(item.full_name.clone(), item);
        }
        Ok(Self { refs })
    }

    /// Looks up the configured upstream; `None` when that ref no longer exists.
    pub fn upstream_of(&self, item: &GitRef) -> Option<&GitRef> {
        self.refs.get(item.upstream.as_deref()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(full: &str, short: &str, upstream: Option<&str>) -> GitRef {
        GitRef {
            full_name: full.into(),
            name: short.into(),
            oid: "a".into(),
            kind: RefKind::Local,
            upstream: upstream.map(Into::into),
        }
    }

    #[test]
    fn upstream_lookup_detects_missing_refs() {
        let mut refs = BTreeMap::new();
        let main = item("refs/heads/main", "main", Some("refs/remotes/origin/main"));
        let gone = item("refs/heads/old", "old", Some("refs/remotes/origin/old"));
        refs.insert(main.full_name.clone(), main.clone());
        refs.insert(
            "refs/remotes/origin/main".into(),
            item("refs/remotes/origin/main", "origin/main", None),
        );
        let set = RefSet { refs };
        assert_eq!(set.upstream_of(&main).unwrap().name, "origin/main");
        assert!(set.upstream_of(&gone).is_none());
    }
}
