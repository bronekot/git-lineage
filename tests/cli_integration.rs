use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{Duration, Instant},
};

use tempfile::TempDir;

struct Repo {
    _temp: TempDir,
    path: PathBuf,
    language_config: PathBuf,
    commit_index: usize,
}

impl Repo {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("repo");
        let language_config = temp.path().join("global.gitconfig");
        fs::create_dir(&path).unwrap();
        git(&path, &["init", "-b", "main"]);
        git(&path, &["config", "user.name", "Lineage Tests"]);
        git(&path, &["config", "user.email", "lineage@example.invalid"]);
        let mut repo = Self {
            _temp: temp,
            path,
            language_config,
            commit_index: 0,
        };
        repo.commit("initial");
        repo
    }

    fn commit(&mut self, message: &str) {
        self.commit_index += 1;
        fs::write(
            self.path.join("history.txt"),
            format!("{}\n", self.commit_index),
        )
        .unwrap();
        git(&self.path, &["add", "history.txt"]);
        git(&self.path, &["commit", "-m", message]);
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_git-lineage"))
            .args(args)
            .current_dir(&self.path)
            .env("GIT_CONFIG_GLOBAL", &self.language_config)
            .output()
            .unwrap()
    }
}

fn git(path: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(path)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

fn stdout(output: &Output) -> String {
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout.clone()).unwrap()
}

#[test]
fn factual_topology_includes_every_ref_groups_oids_and_reduces_transitive_edges() {
    let repo = Repo::new();
    let tree = git(&repo.path, &["rev-parse", "main^{tree}"]);
    let main = git(&repo.path, &["rev-parse", "main"]);
    let commit = |parent: &str, message: &str| {
        git(
            &repo.path,
            &["commit-tree", &tree, "-p", parent, "-m", message],
        )
    };
    let hls = commit(&main, "hls");
    let ui = commit(&hls, "ui");
    let remote_hls = commit(&ui, "remote hls");
    let load = commit(&remote_hls, "load");
    let google = commit(&load, "google");
    let remote_ui = commit(&google, "remote ui");
    let dev = commit(&remote_ui, "dev");
    let ui_ux = commit(&dev, "ui ux");

    for (name, oid) in [
        ("refs/heads/hls-fix", &hls),
        ("refs/heads/ui-fix", &ui),
        ("refs/heads/dev", &dev),
        ("refs/heads/ui-ux-fix", &ui_ux),
        ("refs/remotes/origin/main", &main),
        ("refs/remotes/origin/hls-fix", &remote_hls),
        ("refs/remotes/origin/load-optimization", &load),
        ("refs/remotes/origin/google-fix", &google),
        ("refs/remotes/origin/ui-fix", &remote_ui),
        ("refs/remotes/origin/dev", &dev),
        ("refs/remotes/origin/player-fix", &dev),
    ] {
        git(&repo.path, &["update-ref", name, oid]);
    }
    git(&repo.path, &["switch", "ui-ux-fix"]);

    let text = stdout(&repo.run(&["--no-fetch", "--summary", "none"]));
    let expected = [
        "main (= origin/main)",
        "hls-fix",
        "ui-fix",
        "origin/hls-fix",
        "origin/load-optimization",
        "origin/google-fix",
        "origin/ui-fix",
        "dev (= origin/dev, origin/player-fix)",
        "ui-ux-fix (current)",
    ];
    let mut cursor = 0;
    for name in expected {
        let offset = text[cursor..]
            .find(name)
            .unwrap_or_else(|| panic!("missing {name:?} in:\n{text}"));
        cursor += offset + name.len();
    }
    assert!(
        text.contains(
            "Refs: 12, nodes: 9 (coincident refs merged: 3), edges after transitive reduction: 8."
        ),
        "{text}"
    );

    let report: serde_json::Value = serde_json::from_str(&stdout(&repo.run(&[
        "--no-fetch",
        "--summary",
        "none",
        "--format",
        "json",
    ])))
    .unwrap();
    assert_eq!(report["refs"]["discovered"], 12);
    assert_eq!(report["refs"]["nodes"], 9);
    assert_eq!(report["refs"]["merged_aliases"], 3);
    assert_eq!(report["refs"]["reduced_edges"], 8);
    assert_eq!(report["nodes"].as_array().unwrap().len(), 9);
    assert_eq!(report["relations"].as_array().unwrap().len(), 36);

    let relation = report["relations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| {
            item["left"] == "origin/hls-fix" && item["right"] == "origin/load-optimization"
        })
        .unwrap();
    assert_eq!(relation["left_only"], 0);
    assert_eq!(relation["right_only"], 1);
    assert_eq!(relation["relation"], "left_ancestor");
    for relation in report["relations"].as_array().unwrap() {
        let range = format!(
            "{}...{}",
            relation["left"].as_str().unwrap(),
            relation["right"].as_str().unwrap()
        );
        assert_eq!(
            git(&repo.path, &["rev-list", "--left-right", "--count", &range]),
            format!("{}\t{}", relation["left_only"], relation["right_only"]),
            "{range}"
        );
    }
}

#[test]
fn diverged_refs_are_siblings_with_objective_counts() {
    let mut repo = Repo::new();
    git(&repo.path, &["switch", "-c", "left"]);
    repo.commit("left");
    git(&repo.path, &["switch", "main"]);
    git(&repo.path, &["switch", "-c", "right"]);
    repo.commit("right");

    let text = stdout(&repo.run(&["--no-fetch", "--summary", "brief"]));
    assert!(text.contains("main\n├─ left"), "{text}");
    assert!(text.contains("└─ right (current)"), "{text}");
    assert!(
        text.contains(
            "left and right diverged: left has 1 unique commit, while right has 1 unique commit."
        ),
        "{text}"
    );
    assert!(
        text.contains("Nearest displayed common base: main."),
        "{text}"
    );
    assert_eq!(
        git(
            &repo.path,
            &["rev-list", "--left-right", "--count", "left...right"]
        ),
        "1\t1"
    );
}

#[test]
fn merge_node_keeps_both_immediate_history_parents() {
    let repo = Repo::new();
    let tree = git(&repo.path, &["rev-parse", "main^{tree}"]);
    let main = git(&repo.path, &["rev-parse", "main"]);
    let left = git(
        &repo.path,
        &["commit-tree", &tree, "-p", &main, "-m", "left"],
    );
    let right = git(
        &repo.path,
        &["commit-tree", &tree, "-p", &main, "-m", "right"],
    );
    let merge = git(
        &repo.path,
        &[
            "commit-tree",
            &tree,
            "-p",
            &left,
            "-p",
            &right,
            "-m",
            "merge",
        ],
    );
    for (name, oid) in [("left", &left), ("right", &right), ("merge", &merge)] {
        git(&repo.path, &["branch", name, oid]);
    }

    let report: serde_json::Value = serde_json::from_str(&stdout(&repo.run(&[
        "--no-fetch",
        "--summary",
        "none",
        "--format",
        "json",
    ])))
    .unwrap();
    let merge_node = report["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["name"] == "merge")
        .unwrap();
    let parents: Vec<_> = merge_node["parents"]
        .as_array()
        .unwrap()
        .iter()
        .map(|parent| parent["name"].as_str().unwrap())
        .collect();
    assert_eq!(parents, ["left", "right"]);

    let text = stdout(&repo.run(&["--no-fetch", "--summary", "none"]));
    assert!(text.contains("merge (already shown above)"), "{text}");
    assert!(
        text.contains("edges after transitive reduction: 4."),
        "{text}"
    );
}

#[test]
fn unrelated_histories_are_rendered_as_separate_roots() {
    let repo = Repo::new();
    let tree = git(&repo.path, &["rev-parse", "main^{tree}"]);
    let orphan = git(&repo.path, &["commit-tree", &tree, "-m", "orphan"]);
    git(&repo.path, &["branch", "experiment", &orphan]);
    fs::write(repo.path.join(".git-lineage.toml"), "this is ignored").unwrap();

    let report: serde_json::Value = serde_json::from_str(&stdout(&repo.run(&[
        "--no-fetch",
        "--summary",
        "none",
        "--format",
        "json",
    ])))
    .unwrap();
    assert_eq!(report["refs"]["nodes"], 2);
    assert_eq!(report["roots"].as_array().unwrap().len(), 2);

    let removed_command = repo.run(&["init"]);
    assert_eq!(removed_command.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&removed_command.stderr).contains("unrecognized subcommand"));
}

#[test]
fn tracking_relation_reports_remote_ahead() {
    let mut repo = Repo::new();
    let bare = repo._temp.path().join("remote.git");
    fs::create_dir(&bare).unwrap();
    git(&bare, &["init", "--bare"]);
    let bare_text = bare.to_string_lossy().into_owned();
    git(&repo.path, &["remote", "add", "origin", &bare_text]);
    git(&repo.path, &["push", "-u", "origin", "main"]);
    git(&repo.path, &["switch", "-c", "dev"]);
    repo.commit("dev");
    git(&repo.path, &["push", "-u", "origin", "dev"]);

    let peer = repo._temp.path().join("peer");
    let peer_text = peer.to_string_lossy().into_owned();
    git(repo._temp.path(), &["clone", &bare_text, &peer_text]);
    git(&peer, &["config", "user.name", "Peer"]);
    git(&peer, &["config", "user.email", "peer@example.invalid"]);
    git(&peer, &["switch", "dev"]);
    fs::write(peer.join("peer.txt"), "remote change").unwrap();
    git(&peer, &["add", "peer.txt"]);
    git(&peer, &["commit", "-m", "remote dev"]);
    git(&peer, &["push", "origin", "dev"]);
    git(&repo.path, &["fetch", "origin"]);

    let text = stdout(&repo.run(&["--no-fetch", "--summary", "brief"]));
    assert!(
        text.contains("Local branch dev is 1 commit behind origin/dev"),
        "{text}"
    );

    git(&repo.path, &["push", "origin", "--delete", "dev"]);
    git(&repo.path, &["fetch", "--prune", "origin"]);
    let text = stdout(&repo.run(&["--summary", "brief"]));
    assert!(text.starts_with("Fetched origin;"), "{text}");
    assert!(
        text.contains("Local branch dev tracks origin/dev, which no longer exists."),
        "{text}"
    );
}

#[test]
fn missing_remote_is_reported_without_a_fetch_error() {
    let repo = Repo::new();
    let text = stdout(&repo.run(&[]));
    assert!(
        text.starts_with("Remote origin is not configured; factual ref graph from local data:"),
        "{text}"
    );

    let output = repo.run(&["--format", "json"]);
    let report: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert_eq!(report["fetch"]["status"], "missing_remote");
    assert!(String::from_utf8_lossy(&output.stderr).contains("Remote origin is not configured"));
}

#[test]
fn brief_summary_explains_only_the_current_branch_divergence() {
    let mut repo = Repo::new();
    for name in ["a", "b", "c"] {
        git(&repo.path, &["switch", "-c", name, "main"]);
        repo.commit(name);
    }

    let text = stdout(&repo.run(&["--no-fetch"]));
    assert!(text.contains("a and c diverged"), "{text}");
    assert!(text.contains("b and c diverged"), "{text}");
    assert!(!text.contains("a and b diverged"), "{text}");
    assert!(
        text.contains("Other diverged ref pairs: 1; use --summary full to describe them."),
        "{text}"
    );

    let text = stdout(&repo.run(&["--no-fetch", "--summary", "full"]));
    assert!(text.contains("a and b diverged"), "{text}");
    assert!(!text.contains("Other diverged ref pairs"), "{text}");
}

#[test]
fn language_defaults_to_english_and_persists() {
    let temp = tempfile::tempdir().unwrap();
    let global_config = temp.path().join("global.gitconfig");
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_git-lineage"))
            .args(args)
            .current_dir(temp.path())
            .env("GIT_CONFIG_GLOBAL", &global_config)
            .output()
            .unwrap()
    };
    assert!(stdout(&run(&["language"])).contains("Language: English (en)."));
    assert!(stdout(&run(&["--help"])).contains("Transitively reduced history-inclusion graph"));
    assert!(stdout(&run(&["language", "ru"])).contains("Язык сохранён: русский"));
    assert!(stdout(&run(&["language"])).contains("Язык: русский (ru)."));
    assert!(stdout(&run(&["--help"])).contains("Транзитивно сокращённый граф"));
}

#[test]
fn outside_repository_uses_exit_code_three() {
    let temp = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_git-lineage"))
        .arg("--no-fetch")
        .current_dir(temp.path())
        .env("GIT_CONFIG_GLOBAL", temp.path().join("global.gitconfig"))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
}

#[test]
fn graph_scales_to_one_hundred_refs() {
    let repo = Repo::new();
    let tree = git(&repo.path, &["rev-parse", "main^{tree}"]);
    for index in 0..100 {
        let name = format!("branch-{index:03}");
        let message = format!("branch commit {index}");
        let oid = git(
            &repo.path,
            &["commit-tree", &tree, "-p", "main", "-m", &message],
        );
        git(&repo.path, &["branch", &name, &oid]);
    }
    let started = Instant::now();
    let text = stdout(&repo.run(&["--no-fetch", "--summary", "none"]));
    let elapsed = started.elapsed();
    assert!(text.contains("branch-099"));
    assert!(text.contains("Refs: 101, nodes: 101"), "{text}");
    assert!(elapsed < Duration::from_secs(10), "report took {elapsed:?}");
}
