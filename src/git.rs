use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    process::{Command, Output},
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

    pub fn output<I, S>(&self, args: I) -> Result<Output>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        Command::new("git")
            .args(args)
            .current_dir(&self.root)
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .map_err(LineageError::GitIo)
    }

    pub fn checked<I, S>(&self, args: I) -> Result<Vec<u8>>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let args: Vec<_> = args.into_iter().map(|s| s.as_ref().to_owned()).collect();
        let output = self.output(&args)?;
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
        let message = safe_git_message(&output.stderr);
        Err(LineageError::Git {
            command,
            status: output.status,
            message,
        })
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

    pub fn fetch(&self, remote: &str) -> Result<()> {
        self.checked(["fetch", "--prune", "--quiet", "--", remote])?;
        Ok(())
    }

    pub fn remote_exists(&self, remote: &str) -> Result<bool> {
        let output = self.output(["remote", "get-url", "--", remote])?;
        Ok(output.status.success())
    }

    pub fn is_shallow(&self) -> Result<bool> {
        Ok(self.checked_text(["rev-parse", "--is-shallow-repository"])? == "true")
    }

    pub fn current_branch(&self) -> Result<Option<String>> {
        let output = self.output(["symbolic-ref", "--quiet", "--short", "HEAD"])?;
        if output.status.success() {
            Ok(Some(
                String::from_utf8_lossy(&output.stdout).trim().to_owned(),
            ))
        } else {
            Ok(None)
        }
    }

    pub fn head_oid(&self) -> Result<String> {
        self.checked_text(["rev-parse", "--short", "HEAD"])
    }
}

fn safe_git_message(stderr: &[u8]) -> String {
    // Git can include credential-bearing URLs in diagnostics. Keep only a generic,
    // single-line reason and suppress anything resembling a URL or a secret.
    let raw = String::from_utf8_lossy(stderr);
    raw.lines()
        .find(|line| !line.contains("://") && !line.to_ascii_lowercase().contains("token"))
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .unwrap_or("Git command failed")
        .to_owned()
}
