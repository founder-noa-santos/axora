//! Alternate `GIT_INDEX_FILE` commits: stage blob from memory without touching the working tree.
//!
//! Creates a **dangling** commit (parent = `HEAD`) and does not update `HEAD` or branches.
//! The user can `git cherry-pick` / `git merge` the returned OID later.

use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result};

/// Read `HEAD:path` as UTF-8. Returns `None` if missing or not valid UTF-8.
pub fn read_head_path(repo_root: &Path, relative_unix_path: &str) -> Option<String> {
    if !repo_root.join(".git").exists() {
        return None;
    }
    let out = Command::new("git")
        .current_dir(repo_root)
        .args(["show", &format!("HEAD:{relative_unix_path}")])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8(out.stdout).ok()
}

/// Stage `new_content` for `relative_unix_path` on top of `HEAD`'s tree via an alternate index,
/// then create a commit with `message`. Returns the new commit OID, or `None` if not a git repo.
pub fn try_commit_text_at_head(
    repo_root: &Path,
    relative_unix_path: &str,
    new_content: &str,
    message: &str,
) -> Result<Option<String>> {
    if !repo_root.join(".git").exists() {
        return Ok(None);
    }

    let index_path = repo_root.join(".openakta/git/index.livingdocs");
    if let Some(parent) = index_path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let _ = std::fs::remove_file(&index_path);

    let parent = match git_stdout_trim(repo_root, None, &["rev-parse", "HEAD"]) {
        Ok(h) => h,
        Err(_) => return Ok(None),
    };

    let blob_sha = git_hash_object_stdin(repo_root, new_content.as_bytes())?;

    git_run(
        repo_root,
        Some(index_path.as_path()),
        &["read-tree", &parent],
    )?;

    let cache = format!("100644,{blob_sha},{relative_unix_path}");
    git_run(
        repo_root,
        Some(index_path.as_path()),
        &["update-index", "--add", "--cacheinfo", &cache],
    )?;

    let tree = git_stdout_trim(repo_root, Some(index_path.as_path()), &["write-tree"])?;

    let mut cmd = Command::new("git");
    cmd.current_dir(repo_root);
    cmd.args(["commit-tree", &tree, "-p", &parent, "-m", message]);
    let out = cmd.output().context("git commit-tree")?;
    if !out.status.success() {
        anyhow::bail!(
            "git commit-tree failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let commit = String::from_utf8_lossy(&out.stdout).trim().to_string();

    Ok(Some(commit))
}

fn git_run(repo_root: &Path, index: Option<&Path>, args: &[&str]) -> Result<()> {
    let mut cmd = Command::new("git");
    cmd.current_dir(repo_root);
    if let Some(idx) = index {
        cmd.env("GIT_INDEX_FILE", idx);
    }
    cmd.args(args);
    let out = cmd.output().with_context(|| format!("git {}", args.join(" ")))?;
    if !out.status.success() {
        anyhow::bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(())
}

fn git_stdout_trim(repo_root: &Path, index: Option<&Path>, args: &[&str]) -> Result<String> {
    let mut cmd = Command::new("git");
    cmd.current_dir(repo_root);
    if let Some(idx) = index {
        cmd.env("GIT_INDEX_FILE", idx);
    }
    cmd.args(args);
    let out = cmd.output().with_context(|| format!("git {}", args.join(" ")))?;
    if !out.status.success() {
        anyhow::bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn git_hash_object_stdin(repo_root: &Path, bytes: &[u8]) -> Result<String> {
    let mut child = Command::new("git")
        .current_dir(repo_root)
        .args(["hash-object", "-w", "--stdin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .context("git hash-object")?;
    {
        use std::io::Write;
        let stdin = child.stdin.as_mut().expect("stdin");
        stdin.write_all(bytes)?;
    }
    let out = child.wait_with_output().context("git hash-object wait")?;
    if !out.status.success() {
        anyhow::bail!(
            "git hash-object failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;

    #[test]
    fn alternate_index_commit_leaves_worktree_unchanged() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path();
        Command::new("git")
            .args(["init"])
            .current_dir(repo)
            .status()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "t@e.st"])
            .current_dir(repo)
            .status()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "test"])
            .current_dir(repo)
            .status()
            .unwrap();
        fs::write(repo.join("doc.md"), "# Hi\n").unwrap();
        Command::new("git")
            .args(["add", "doc.md"])
            .current_dir(repo)
            .status()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "init"])
            .current_dir(repo)
            .status()
            .unwrap();

        let before = fs::read_to_string(repo.join("doc.md")).unwrap();
        let oid = try_commit_text_at_head(
            repo,
            "doc.md",
            "# Hi\n\n<!-- openakta:changelog -->\n\n- `x`\n",
            "livingdocs test",
        )
        .unwrap()
        .expect("commit");
        assert_eq!(fs::read_to_string(repo.join("doc.md")).unwrap(), before);
        assert!(!oid.is_empty());
    }

    #[test]
    fn try_commit_returns_none_when_repository_has_no_head() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path();
        Command::new("git")
            .args(["init"])
            .current_dir(repo)
            .status()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "t@e.st"])
            .current_dir(repo)
            .status()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "test"])
            .current_dir(repo)
            .status()
            .unwrap();

        let r = try_commit_text_at_head(repo, "x.md", "body", "msg").unwrap();
        assert!(r.is_none(), "expected graceful skip when HEAD does not exist");
    }
}
