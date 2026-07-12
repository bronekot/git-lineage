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

    fn config(&self, text: &str) {
        fs::write(self.path.join(".git-lineage.toml"), text).unwrap();
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
fn language_defaults_to_english_and_persists_across_processes() {
    let temp = tempfile::tempdir().unwrap();
    let global_config = temp.path().join("global.gitconfig");
    let run = |cwd: &Path, args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_git-lineage"))
            .args(args)
            .current_dir(cwd)
            .env("GIT_CONFIG_GLOBAL", &global_config)
            .output()
            .unwrap()
    };

    let default_language = stdout(&run(temp.path(), &["language"]));
    assert!(default_language.contains("Language: English (en)."));
    let english_help = stdout(&run(temp.path(), &["--help"]));
    assert!(english_help.contains("Compact semantic branch trees"));
    assert!(english_help.contains("Show synced upstream branches"));
    let english_error = run(temp.path(), &["--no-fetch"]);
    assert_eq!(english_error.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&english_error.stderr)
        .contains("Error: the command was run outside a Git repository"));

    let saved_russian = stdout(&run(temp.path(), &["language", "ru"]));
    assert!(saved_russian.contains("Язык сохранён: русский (ru)."));
    let reloaded_russian = stdout(&run(temp.path(), &["language"]));
    assert!(reloaded_russian.contains("Язык: русский (ru)."));
    let russian_help = stdout(&run(temp.path(), &["--help"]));
    assert!(russian_help.contains("Компактное семантическое дерево веток Git"));
    assert!(russian_help.contains("Удалённый репозиторий для получения данных и отчёта"));
    let russian_error = run(temp.path(), &["--no-fetch"]);
    assert_eq!(russian_error.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&russian_error.stderr)
        .contains("Ошибка: команда запущена вне Git-репозитория"));

    let mut repo = Repo::new();
    git(&repo.path, &["switch", "-c", "dev"]);
    repo.commit("dev");
    let russian_report = stdout(&run(&repo.path, &["--no-fetch", "--summary", "full"]));
    assert!(russian_report.contains("Граф веток по локальным данным:"));
    assert!(russian_report.contains("dev (текущая)"));
    assert!(russian_report.contains("+1 от main"));
    assert!(russian_report.contains("Вероятный родитель ветки dev — main"));
    assert!(russian_report.contains("расстояние — 1 коммит."));

    let saved_english = stdout(&run(temp.path(), &["language", "en"]));
    assert!(saved_english.contains("Language saved: English (en)."));
    let english_report = stdout(&run(&repo.path, &["--no-fetch", "--summary", "full"]));
    assert!(english_report.contains("Branch graph from local data:"));
    assert!(english_report.contains("dev (current)"));
    assert!(english_report.contains("+1 from main"));
    assert!(english_report.contains("Likely parent of dev: main"));
    assert!(english_report.contains("1 commit away"));
}

#[test]
fn renders_linear_tree_and_json() {
    let mut repo = Repo::new();
    git(&repo.path, &["switch", "-c", "dev"]);
    repo.commit("dev");
    git(&repo.path, &["switch", "-c", "feature"]);
    repo.commit("feature");
    repo.config(
        "version = 1\nroot = \"main\"\nremote = \"origin\"\nfetch = false\n\n[parents]\ndev = \"main\"\nfeature = \"dev\"\n",
    );

    let head_before = git(&repo.path, &["rev-parse", "HEAD"]);
    let status_before = git(&repo.path, &["status", "--porcelain"]);
    let text = stdout(&repo.run(&["--no-fetch", "--summary", "none"]));
    assert!(text.contains("main\n└─ dev"), "{text}");
    assert!(text.contains("└─ feature (current)"), "{text}");
    assert!(text.contains("+1 from main"), "{text}");

    let json = stdout(&repo.run(&["report", "--no-fetch", "--format", "json"]));
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["root"], "main");
    assert_eq!(value["current_branch"], "feature");
    assert_eq!(value["nodes"].as_array().unwrap().len(), 3);
    assert_eq!(git(&repo.path, &["rev-parse", "HEAD"]), head_before);
    assert_eq!(git(&repo.path, &["status", "--porcelain"]), status_before);

    let binary = PathBuf::from(env!("CARGO_BIN_EXE_git-lineage"));
    let mut paths = vec![binary.parent().unwrap().to_path_buf()];
    paths.extend(env::split_paths(&env::var_os("PATH").unwrap_or_default()));
    let via_git = Command::new("git")
        .args(["lineage", "--no-fetch", "--summary", "none"])
        .current_dir(&repo.path)
        .env("PATH", env::join_paths(paths).unwrap())
        .env("GIT_CONFIG_GLOBAL", &repo.language_config)
        .output()
        .unwrap();
    assert!(via_git.status.success());
    assert!(String::from_utf8_lossy(&via_git.stdout).contains("feature (current)"));

    let colored = stdout(&repo.run(&["--no-fetch", "--color", "always", "--summary", "none"]));
    assert!(colored.contains("\u{1b}[1;36mfeature\u{1b}[0m"));
    let plain = stdout(&repo.run(&["--no-fetch", "--color", "never", "--summary", "none"]));
    assert!(!plain.contains("\u{1b}["));
}

#[test]
fn report_discovers_missing_parents_conservatively() {
    let mut repo = Repo::new();
    git(&repo.path, &["switch", "-c", "dev"]);
    repo.commit("dev");
    git(&repo.path, &["switch", "-c", "feature"]);
    repo.commit("feature");

    let text = stdout(&repo.run(&["--no-fetch", "--summary", "full"]));
    assert!(text.contains("main\n└─ dev"), "{text}");
    assert!(text.contains("└─ feature (current)"), "{text}");
    assert!(text.contains("Likely parent of dev: main"), "{text}");
    assert!(text.contains("Likely parent of feature: dev"), "{text}");

    repo.config(
        "version = 1\nroot = \"main\"\nfetch = false\n\n[parents]\ndev = \"main\"\nfeature = \"main\"\n",
    );
    let discovered = stdout(&repo.run(&["discover", "--no-fetch"]));
    assert!(discovered.contains("feature = \"main\""), "{discovered}");
}

#[test]
fn compares_sibling_leaf_branches() {
    let mut repo = Repo::new();
    git(&repo.path, &["switch", "-c", "dev"]);
    repo.commit("dev");
    git(&repo.path, &["switch", "-c", "лево"]);
    repo.commit("left");
    git(&repo.path, &["switch", "dev"]);
    git(&repo.path, &["switch", "-c", "right"]);
    repo.commit("right");
    repo.config(
        "version = 1\nroot = \"main\"\nfetch = false\n\n[parents]\ndev = \"main\"\n\"лево\" = \"dev\"\nright = \"dev\"\n",
    );
    let text = stdout(&repo.run(&["--no-fetch"]));
    assert!(
        text.contains("right and лево are sibling branches from dev"),
        "{text}"
    );
    assert!(text.contains("1 unique commit"), "{text}");
}

#[test]
fn collapses_synced_upstream_and_shows_differing_upstream() {
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

    repo.config(
        "version = 1\nroot = \"main\"\nremote = \"origin\"\nfetch = false\n\n[parents]\ndev = \"main\"\n",
    );
    let text = stdout(&repo.run(&["--no-fetch"]));
    assert!(text.contains("└─ origin/dev"), "{text}");
    assert!(
        text.contains("Local branch dev is 1 commit behind origin/dev"),
        "{text}"
    );
    assert!(
        !text.contains("─ origin/main"),
        "synced main must be collapsed: {text}"
    );
    assert!(
        text.contains("main matches its counterpart on origin"),
        "{text}"
    );

    let shown = stdout(&repo.run(&["--no-fetch", "--show-synced-remotes", "--summary", "none"]));
    assert!(shown.contains("origin/main"), "{shown}");

    let missing = repo._temp.path().join("missing-remote.git");
    let missing_text = missing.to_string_lossy().into_owned();
    git(&repo.path, &["remote", "set-url", "origin", &missing_text]);
    repo.config(
        "version = 1\nroot = \"main\"\nremote = \"origin\"\nfetch = true\n\n[parents]\ndev = \"main\"\n",
    );
    let cached = stdout(&repo.run(&[]));
    assert!(cached.contains("Could not fetch origin"), "{cached}");
    assert!(cached.contains("origin/dev"), "{cached}");

    repo.config(
        "version = 1\nroot = \"main\"\nremote = \"origin\"\nfetch = false\n\n[parents]\n\"origin/main\" = \"dev\"\n",
    );
    assert_eq!(repo.run(&["--no-fetch"]).status.code(), Some(2));
}

#[test]
fn reports_local_ahead_and_diverged_tracking_states() {
    let mut repo = Repo::new();
    let bare = repo._temp.path().join("remote-diverged.git");
    fs::create_dir(&bare).unwrap();
    git(&bare, &["init", "--bare"]);
    let bare_text = bare.to_string_lossy().into_owned();
    git(&repo.path, &["remote", "add", "origin", &bare_text]);
    git(&repo.path, &["push", "-u", "origin", "main"]);
    git(&repo.path, &["switch", "-c", "dev"]);
    repo.commit("dev");
    git(&repo.path, &["push", "-u", "origin", "dev"]);

    repo.commit("local only");
    let ahead = stdout(&repo.run(&["--no-fetch"]));
    assert!(
        ahead.contains("Local branch dev is 1 commit ahead of origin/dev"),
        "{ahead}"
    );

    stdout(&repo.run(&["language", "ru"]));
    let ahead_ru = stdout(&repo.run(&["--no-fetch"]));
    assert!(
        ahead_ru.contains("Локальная ветка dev опережает origin/dev на 1 коммит"),
        "{ahead_ru}"
    );
    stdout(&repo.run(&["language", "en"]));

    let peer = repo._temp.path().join("peer-diverged");
    let peer_text = peer.to_string_lossy().into_owned();
    git(repo._temp.path(), &["clone", &bare_text, &peer_text]);
    git(&peer, &["config", "user.name", "Peer"]);
    git(&peer, &["config", "user.email", "peer@example.invalid"]);
    git(&peer, &["switch", "dev"]);
    fs::write(peer.join("remote.txt"), "remote only").unwrap();
    git(&peer, &["add", "remote.txt"]);
    git(&peer, &["commit", "-m", "remote only"]);
    git(&peer, &["push", "origin", "dev"]);
    git(&repo.path, &["fetch", "origin"]);

    let diverged = stdout(&repo.run(&["--no-fetch"]));
    assert!(
        diverged.contains("dev and origin/dev diverged"),
        "{diverged}"
    );
    assert!(diverged.contains("dev has 1 unique commit"), "{diverged}");
}

#[test]
fn full_report_includes_remote_only_and_fetch_prunes_deleted_ref() {
    let repo = Repo::new();
    let bare = repo._temp.path().join("remote-prune.git");
    fs::create_dir(&bare).unwrap();
    git(&bare, &["init", "--bare"]);
    let bare_text = bare.to_string_lossy().into_owned();
    git(&repo.path, &["remote", "add", "origin", &bare_text]);
    git(&repo.path, &["push", "-u", "origin", "main"]);

    let peer = repo._temp.path().join("peer-prune");
    let peer_text = peer.to_string_lossy().into_owned();
    git(repo._temp.path(), &["clone", &bare_text, &peer_text]);
    git(&peer, &["config", "user.name", "Peer"]);
    git(&peer, &["config", "user.email", "peer@example.invalid"]);
    git(&peer, &["switch", "-c", "remote-only", "origin/main"]);
    fs::write(peer.join("remote-only.txt"), "remote").unwrap();
    git(&peer, &["add", "remote-only.txt"]);
    git(&peer, &["commit", "-m", "remote-only"]);
    git(&peer, &["push", "-u", "origin", "remote-only"]);
    git(&repo.path, &["fetch", "origin"]);
    repo.config(
        "version = 1\nroot = \"main\"\nremote = \"origin\"\nfetch = true\n\n[display]\nshow_remote_only = true\nsummary = \"full\"\n",
    );

    let before = stdout(&repo.run(&["--no-fetch", "--format", "json"]));
    let value: serde_json::Value = serde_json::from_str(&before).unwrap();
    assert!(value["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|node| node["name"] == "origin/remote-only"));

    git(&peer, &["push", "origin", "--delete", "remote-only"]);
    let after = stdout(&repo.run(&["--summary", "full"]));
    assert!(after.contains("Fetched origin"), "{after}");
    assert!(!after.contains("remote-only"), "{after}");
}

#[test]
fn fetch_failure_without_remote_is_non_fatal_and_shallow_clone_warns() {
    let mut source = Repo::new();
    source.commit("second");
    let no_remote = source.run(&[]);
    assert!(
        no_remote.status.success(),
        "{}",
        String::from_utf8_lossy(&no_remote.stderr)
    );
    assert!(String::from_utf8_lossy(&no_remote.stdout).contains("Could not fetch origin"));
    let json_failure = source.run(&["--format", "json"]);
    assert!(json_failure.status.success());
    serde_json::from_slice::<serde_json::Value>(&json_failure.stdout).unwrap();
    assert!(String::from_utf8_lossy(&json_failure.stderr).contains("Could not fetch origin"));

    stdout(&source.run(&["language", "ru"]));
    let no_remote_ru = stdout(&source.run(&[]));
    assert!(
        no_remote_ru
            .contains("Не удалось получить данные из origin; граф построен по сохранённым данным:"),
        "{no_remote_ru}"
    );
    stdout(&source.run(&["language", "en"]));

    let shallow = source._temp.path().join("shallow");
    let shallow_text = shallow.to_string_lossy().into_owned();
    let source_uri = format!(
        "file:///{}",
        source.path.to_string_lossy().replace('\\', "/")
    );
    git(
        source._temp.path(),
        &["clone", "--depth", "1", &source_uri, &shallow_text],
    );
    let output = Command::new(env!("CARGO_BIN_EXE_git-lineage"))
        .arg("--no-fetch")
        .current_dir(&shallow)
        .env("GIT_CONFIG_GLOBAL", &source.language_config)
        .output()
        .unwrap();
    let text = stdout(&output);
    assert!(
        text.contains("Repository history is limited by a shallow clone"),
        "{text}"
    );

    stdout(&source.run(&["language", "ru"]));
    let russian_output = Command::new(env!("CARGO_BIN_EXE_git-lineage"))
        .arg("--no-fetch")
        .current_dir(&shallow)
        .env("GIT_CONFIG_GLOBAL", &source.language_config)
        .output()
        .unwrap();
    let russian_text = stdout(&russian_output);
    assert!(
        russian_text.contains("История ограничена из-за неполного (shallow) клона"),
        "{russian_text}"
    );
}

#[test]
fn detached_head_and_ascii_are_reported() {
    let repo = Repo::new();
    git(&repo.path, &["checkout", "--detach", "HEAD"]);
    let text = stdout(&repo.run(&["--no-fetch", "--ascii"]));
    assert!(text.contains("HEAD is detached at"), "{text}");
    assert!(text.contains("there is no current local branch"), "{text}");
    assert!(!text.contains("(current)"), "{text}");

    stdout(&repo.run(&["language", "ru"]));
    let russian = stdout(&repo.run(&["--no-fetch", "--ascii"]));
    assert!(
        russian.contains("HEAD отсоединён от ветки и указывает на коммит"),
        "{russian}"
    );
    assert!(
        russian.contains("текущая локальная ветка не выбрана"),
        "{russian}"
    );
}

#[test]
fn init_parent_discover_and_cycle_errors_have_expected_behavior() {
    let mut repo = Repo::new();
    let init = repo.run(&["init"]);
    assert!(init.status.success());
    assert!(repo.path.join(".git-lineage.toml").exists());
    assert_eq!(repo.run(&["init"]).status.code(), Some(2));
    assert_eq!(repo.run(&["check"]).status.code(), Some(2));

    git(&repo.path, &["switch", "-c", "feature"]);
    repo.commit("feature");
    assert!(repo.run(&["parent", "feature", "main"]).status.success());
    let saved = fs::read_to_string(repo.path.join(".git-lineage.toml")).unwrap();
    assert!(saved.contains("feature = \"main\""));
    assert!(repo.run(&["parent", "feature", "--unset"]).status.success());

    let tree = git(&repo.path, &["rev-parse", "main^{tree}"]);
    let orphan = git(&repo.path, &["commit-tree", &tree, "-m", "orphan"]);
    git(&repo.path, &["branch", "experiment", &orphan]);

    let discovered = stdout(&repo.run(&["discover", "--no-fetch"]));
    assert!(discovered.contains("feature = \"main\""), "{discovered}");
    assert!(discovered.contains("Unplaced branches"), "{discovered}");
    assert!(discovered.contains("- experiment"), "{discovered}");

    repo.config(
        "version = 1\nroot = \"main\"\nfetch = false\n\n[parents]\nmain = \"feature\"\nfeature = \"main\"\n",
    );
    let cycle = repo.run(&["--no-fetch"]);
    assert_eq!(cycle.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&cycle.stderr).contains("cycle"));
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
fn configured_report_scales_to_one_hundred_branches() {
    let repo = Repo::new();
    let tree = git(&repo.path, &["rev-parse", "main^{tree}"]);
    let mut config = String::from(
        "version = 1\nroot = \"main\"\nremote = \"origin\"\nfetch = false\n\n[parents]\n",
    );
    for index in 0..100 {
        let name = format!("branch-{index:03}");
        let message = format!("branch commit {index}");
        let oid = git(
            &repo.path,
            &["commit-tree", &tree, "-p", "main", "-m", &message],
        );
        git(&repo.path, &["branch", &name, &oid]);
        config.push_str(&format!("{name} = \"main\"\n"));
    }
    repo.config(&config);

    let started = Instant::now();
    let text = stdout(&repo.run(&["--no-fetch", "--summary", "brief"]));
    let elapsed = started.elapsed();
    assert!(text.contains("branch-099"));
    assert_eq!(text.matches("sibling branches").count(), 99);
    assert!(
        elapsed < Duration::from_secs(10),
        "100-branch report took {elapsed:?}"
    );
}
