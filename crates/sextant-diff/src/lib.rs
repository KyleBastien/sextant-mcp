//! Git-backed diff acquisition.
//!
//! For `--diff` grading we need three things from a repo:
//!   1. The set of changed paths between `base` and `head`.
//!   2. The line ranges (in the head version) that changed within each file.
//!   3. The current contents of each changed file (to feed to evaluators).
//!
//! `head` is either a git ref (tree-to-tree compare) or the working tree
//! (tree-to-workdir compare). Working tree is the default for local agent
//! flows; ref-to-ref is what the GitHub Action uses.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use git2::{Diff, DiffOptions, Oid, Repository, Tree};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DiffError {
    #[error("git: {0}")]
    Git(#[from] git2::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("ref `{0}` did not resolve to a commit")]
    NotACommit(String),
    #[error("could not determine default base: no `origin/main`, no HEAD~1")]
    NoDefaultBase,
}

pub type DiffResult<T> = Result<T, DiffError>;

/// Selector for the "head" side of a diff.
#[derive(Debug, Clone)]
pub enum HeadSpec {
    /// Diff base-tree → working tree (with index applied).
    WorkingTree,
    /// Diff base-tree → tree-of-ref.
    Ref(String),
}

/// Selector for the "base" side of a diff.
#[derive(Debug, Clone)]
pub enum BaseSpec {
    /// Use `merge-base origin/main HEAD`, falling back to `HEAD~1`.
    Auto,
    /// A user-supplied ref.
    Ref(String),
}

#[derive(Debug, Clone)]
pub struct DiffFile {
    pub path: PathBuf,
    pub status: ChangeKind,
    /// Line ranges (1-based, inclusive) in the *head* version that the diff
    /// touches. For `Added` files this is the entire file.
    pub changed_lines: BTreeSet<u32>,
    /// Current contents of the file in the head version. `None` when the
    /// file was deleted or could not be read as UTF-8.
    pub head_contents: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    Other,
}

#[derive(Debug, Clone)]
pub struct DiffSet {
    pub base_oid: Oid,
    pub head_oid: Option<Oid>,
    pub files: Vec<DiffFile>,
}

impl DiffSet {
    pub fn file_for(&self, path: &Path) -> Option<&DiffFile> {
        self.files.iter().find(|f| f.path == path)
    }
}

/// Resolve a base ref to its tree's commit OID.
fn resolve_base(repo: &Repository, spec: &BaseSpec) -> DiffResult<Oid> {
    match spec {
        BaseSpec::Ref(name) => resolve_commit(repo, name),
        BaseSpec::Auto => auto_base(repo),
    }
}

/// Try `merge-base origin/main HEAD`, then `origin/master`, then `HEAD~1`.
fn auto_base(repo: &Repository) -> DiffResult<Oid> {
    let head = repo.head()?.peel_to_commit()?.id();
    for candidate in ["origin/main", "origin/master"] {
        if let Some(mb) = try_merge_base(repo, head, candidate) {
            return Ok(mb);
        }
    }
    resolve_commit(repo, "HEAD~1").map_err(|_| DiffError::NoDefaultBase)
}

fn try_merge_base(repo: &Repository, head: Oid, candidate: &str) -> Option<Oid> {
    let other = resolve_commit(repo, candidate).ok()?;
    repo.merge_base(head, other).ok()
}

fn resolve_commit(repo: &Repository, name: &str) -> DiffResult<Oid> {
    let obj = repo.revparse_single(name)?;
    let commit = obj
        .peel_to_commit()
        .map_err(|_| DiffError::NotACommit(name.to_string()))?;
    Ok(commit.id())
}

fn tree_for(repo: &Repository, oid: Oid) -> DiffResult<Tree<'_>> {
    let commit = repo.find_commit(oid)?;
    Ok(commit.tree()?)
}

/// Compute a diff against the working tree (with index applied) or against
/// a head ref's tree.
pub fn compute(repo_root: &Path, base: &BaseSpec, head: &HeadSpec) -> DiffResult<DiffSet> {
    let repo = Repository::discover(repo_root)?;
    let base_oid = resolve_base(&repo, base)?;
    let base_tree = tree_for(&repo, base_oid)?;
    let mut diff_opts = default_diff_opts();

    let (diff, head_oid) = build_diff(&repo, &base_tree, head, &mut diff_opts)?;
    let files = collect_files(&repo, repo_root, &diff, head)?;
    Ok(DiffSet {
        base_oid,
        head_oid,
        files,
    })
}

fn default_diff_opts() -> DiffOptions {
    let mut opts = DiffOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .show_untracked_content(true)
        .context_lines(0);
    opts
}

fn build_diff<'repo>(
    repo: &'repo Repository,
    base_tree: &Tree<'repo>,
    head: &HeadSpec,
    opts: &mut DiffOptions,
) -> DiffResult<(Diff<'repo>, Option<Oid>)> {
    match head {
        HeadSpec::WorkingTree => {
            let d = repo.diff_tree_to_workdir_with_index(Some(base_tree), Some(opts))?;
            Ok((d, None))
        }
        HeadSpec::Ref(name) => {
            let head_oid = resolve_commit(repo, name)?;
            let head_tree = tree_for(repo, head_oid)?;
            let d = repo.diff_tree_to_tree(Some(base_tree), Some(&head_tree), Some(opts))?;
            Ok((d, Some(head_oid)))
        }
    }
}

fn delta_path(delta: &git2::DiffDelta<'_>) -> PathBuf {
    delta
        .new_file()
        .path()
        .map(Path::to_path_buf)
        .or_else(|| delta.old_file().path().map(Path::to_path_buf))
        .unwrap_or_default()
}

fn change_kind(status: git2::Delta) -> ChangeKind {
    match status {
        git2::Delta::Added | git2::Delta::Untracked => ChangeKind::Added,
        git2::Delta::Modified | git2::Delta::Typechange => ChangeKind::Modified,
        git2::Delta::Deleted => ChangeKind::Deleted,
        git2::Delta::Renamed => ChangeKind::Renamed,
        git2::Delta::Copied => ChangeKind::Copied,
        _ => ChangeKind::Other,
    }
}

fn collect_files(
    repo: &Repository,
    repo_root: &Path,
    diff: &Diff<'_>,
    head: &HeadSpec,
) -> DiffResult<Vec<DiffFile>> {
    use std::cell::RefCell;
    let acc: RefCell<Vec<DiffFile>> = RefCell::new(Vec::new());

    diff.foreach(
        &mut |delta, _progress| {
            acc.borrow_mut().push(DiffFile {
                path: delta_path(&delta),
                status: change_kind(delta.status()),
                changed_lines: BTreeSet::new(),
                head_contents: None,
            });
            true
        },
        None,
        Some(&mut |delta, hunk| {
            let path = delta_path(&delta);
            let start = hunk.new_start();
            let lines = hunk.new_lines();
            if lines == 0 {
                return true;
            }
            let end = start + lines - 1;
            let mut acc = acc.borrow_mut();
            if let Some(f) = acc.iter_mut().find(|f| f.path == path) {
                f.changed_lines.extend(start..=end);
            }
            true
        }),
        None,
    )?;

    let mut files = acc.into_inner();
    for f in files.iter_mut() {
        if matches!(f.status, ChangeKind::Deleted) {
            continue;
        }
        f.head_contents = read_head(repo, repo_root, &f.path, head)?;
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

fn read_head(
    repo: &Repository,
    repo_root: &Path,
    path: &Path,
    head: &HeadSpec,
) -> DiffResult<Option<String>> {
    match head {
        HeadSpec::WorkingTree => read_workdir(repo_root, path),
        HeadSpec::Ref(name) => read_blob_at_ref(repo, name, path),
    }
}

fn read_workdir(repo_root: &Path, path: &Path) -> DiffResult<Option<String>> {
    let abs = repo_root.join(path);
    match std::fs::read_to_string(&abs) {
        Ok(s) => Ok(Some(s)),
        // NotFound = file deleted in workdir but still in tree; InvalidData
        // = binary or non-UTF-8 content. Both should yield None rather than
        // bubble up an error — the caller treats absent contents as "skip".
        Err(err)
            if err.kind() == std::io::ErrorKind::NotFound
                || err.kind() == std::io::ErrorKind::InvalidData =>
        {
            Ok(None)
        }
        Err(err) => Err(err.into()),
    }
}

fn read_blob_at_ref(repo: &Repository, name: &str, path: &Path) -> DiffResult<Option<String>> {
    let oid = resolve_commit(repo, name)?;
    let tree = tree_for(repo, oid)?;
    let Ok(entry) = tree.get_path(path) else {
        return Ok(None);
    };
    let blob = entry.to_object(repo)?.peel_to_blob()?;
    match std::str::from_utf8(blob.content()) {
        Ok(s) => Ok(Some(s.to_string())),
        Err(_) => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;

    fn git(dir: &Path, args: &[&str]) {
        let status = Command::new("git")
            .args(args)
            .current_dir(dir)
            .status()
            .expect("running git");
        assert!(status.success(), "git {args:?} failed");
    }

    fn init_repo(dir: &Path) {
        git(dir, &["init", "-q", "-b", "main"]);
        git(dir, &["config", "user.email", "test@example.com"]);
        git(dir, &["config", "user.name", "Test"]);
        git(dir, &["config", "commit.gpgsign", "false"]);
    }

    #[test]
    fn working_tree_diff_against_base() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        init_repo(root);

        fs::write(root.join("a.txt"), "alpha\nbeta\n").unwrap();
        git(root, &["add", "."]);
        git(root, &["commit", "-q", "-m", "init"]);

        // Modify the file in the working tree.
        fs::write(root.join("a.txt"), "alpha\nBETA\ngamma\n").unwrap();
        // Add an untracked file.
        fs::write(root.join("b.txt"), "new\n").unwrap();

        let diff =
            compute(root, &BaseSpec::Ref("HEAD".into()), &HeadSpec::WorkingTree).expect("diff");

        let a = diff.file_for(Path::new("a.txt")).expect("a.txt in diff");
        assert_eq!(a.status, ChangeKind::Modified);
        // Line 2 changed; line 3 added.
        assert!(a.changed_lines.contains(&2));
        assert!(a.changed_lines.contains(&3));

        let b = diff.file_for(Path::new("b.txt")).expect("b.txt in diff");
        assert_eq!(b.status, ChangeKind::Added);
        assert!(b.changed_lines.contains(&1));
    }

    #[test]
    fn ref_to_ref_diff() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        init_repo(root);

        fs::write(root.join("a.txt"), "one\n").unwrap();
        git(root, &["add", "."]);
        git(root, &["commit", "-q", "-m", "init"]);
        let base_sha = String::from_utf8(
            Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(root)
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .trim()
        .to_string();

        fs::write(root.join("a.txt"), "one\ntwo\n").unwrap();
        git(root, &["commit", "-aq", "-m", "second"]);

        let diff = compute(
            root,
            &BaseSpec::Ref(base_sha),
            &HeadSpec::Ref("HEAD".into()),
        )
        .expect("diff");

        let a = diff.file_for(Path::new("a.txt")).expect("a.txt in diff");
        assert!(a.changed_lines.contains(&2));
        assert_eq!(a.head_contents.as_deref(), Some("one\ntwo\n"));
    }
}
