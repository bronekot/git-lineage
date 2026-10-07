use std::{
    ffi::OsStr,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    thread,
};

use crate::error::{LineageError, Result};

#[derive(Debug, Clone)]
pub struct GitRepo {
    root: PathBuf,
}

impl GitRepo {
    pub fn discover(start: &Path) -> Result<Self> {
        let output = Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .current_dir(start)
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .map_err(LineageError::GitIo)?;
        if !output.status.success() {
            return Err(LineageError::NotRepository);
        }
        let root = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        Ok(Self {
            root: PathBuf::from(root),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn command<I, S>(&self, args: I) -> Command
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut command = Command::new("git");
        command
            .args(args)
            .current_dir(&self.root)
            .env("GIT_TERMINAL_PROMPT", "0");
        command
    }

    pub fn output<I, S>(&self, args: I) -> Result<Output>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.command(args).output().map_err(LineageError::GitIo)
    }

    pub fn checked<I, S>(&self, args: I) -> Result<Vec<u8>>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let args: Vec<_> = args.into_iter().map(|s| s.as_ref().to_owned()).collect();
        let output = self.output(&args)?;
        check_output(&args, output)
    }

    /// Runs Git with `input` written to its standard input. Large argument
    /// lists go through stdin to stay below command-line length limits.
    pub fn checked_with_input<I, S>(&self, args: I, input: Vec<u8>) -> Result<Vec<u8>>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let args: Vec<_> = args.into_iter().map(|s| s.as_ref().to_owned()).collect();
        let mut child = self
            .command(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(LineageError::GitIo)?;
        let mut stdin = child.stdin.take().expect("stdin was piped");
        let writer = thread::spawn(move || stdin.write_all(&input));
        let output = child.wait_with_output().map_err(LineageError::GitIo)?;
        // A write error only matters if Git itself succeeded.
        if output.status.success() {
            writer
                .join()
                .expect("stdin writer does not panic")
                .map_err(LineageError::GitIo)?;
        }
        check_output(&args, output)
    }

    pub fn checked_text<I, S>(&self, args: I) -> Result<String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        Ok(String::from_utf8_lossy(&self.checked(args)?)
            .trim()
            .to_owned())
    }

    pub fn has_remote(&self, remote: &str) -> Result<bool> {
        let remotes = self.checked_text(["remote"])?;
        Ok(remotes.lines().any(|line| line.trim() == remote))
    }

    pub fn fetch(&self, remote: &str) -> Result<()> {
        self.checked(["fetch", "--prune", "--quiet", "--", remote])?;
        Ok(())
    }

    pub fn is_shallow(&self) -> Result<bool> {
        Ok(self.checked_text(["rev-parse", "--is-shallow-repository"])? == "true")
    }

    /// Full ref name of the checked-out branch, such as `refs/heads/main`.
    pub fn current_branch(&self) -> Result<Option<String>> {
        let output = self.output(["symbolic-ref", "--quiet", "HEAD"])?;
        if !output.status.success() {
            return Ok(None);
        }
        let name = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        Ok(name.starts_with("refs/heads/").then_some(name))
    }

    pub fn head_oid(&self) -> Result<String> {
        self.checked_text(["rev-parse", "--short", "HEAD"])
    }
}

fn check_output(args: &[std::ffi::OsString], output: Output) -> Result<Vec<u8>> {
    if output.status.success() {
        return Ok(output.stdout);
    }
    let command = format!(
        "git {}",
        args.iter()
            .map(|s| s.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ")
    );
    Err(LineageError::Git {
        command,
        status: output.status,
        message: safe_git_message(&output.stderr),
    })
}

fn safe_git_message(stderr: &[u8]) -> String {
    // Git can include credential-bearing URLs in diagnostics. Keep only a generic,
    // single-line reason and suppress anything resembling a URL or a secret.
    let raw = String::from_utf8_lossy(stderr);
    raw.lines()
        .map(str::trim)
        .find(|line| {
            let lower = line.to_ascii_lowercase();
            !line.is_empty()
                && !line.contains("://")
                && !lower.contains("token")
                && !lower.contains("password")
        })
        .unwrap_or("Git command failed")
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_message_skips_blank_and_secret_lines() {
        let stderr = b"\nfatal: unable to access 'https://user:secret@host/repo': 403\nfatal: Authentication failed\n";
        assert_eq!(safe_git_message(stderr), "fatal: Authentication failed");
        assert_eq!(safe_git_message(b"\n  \n"), "Git command failed");
    }
}
