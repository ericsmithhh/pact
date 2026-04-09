//! Project-root discovery and source-file collection.
//!
//! The two primary entry points are:
//!
//! - [`find_project_root`] — walks up the directory tree from a starting path
//!   looking for a `pact.toml` file and returns the directory that contains it.
//! - [`SourceSet::discover`] — given a project root, parses `pact.toml` and
//!   collects all `.pact` files under `src/` and `test/`.
//!
//! # Example
//!
//! ```no_run
//! use std::path::Path;
//! use pact_compiler::project::{find_project_root, SourceSet};
//!
//! let root = find_project_root(Path::new(".")).expect("not inside a Pact project");
//! let sources = SourceSet::discover(&root).expect("failed to collect sources");
//! println!("found {} source files", sources.sources().len());
//! ```
//!
//! TODO: Extract this module into a dedicated `pact-project` crate once the
//! project-model surface area stabilises. That will allow tools (LSP, fmt,
//! CLI) to depend on project discovery without pulling in the full compiler
//! pipeline.

use std::{
    collections::VecDeque,
    fs,
    path::{Path, PathBuf},
};

use thiserror::Error;

use crate::manifest::{Manifest, ManifestError};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur during project-root discovery or source collection.
#[derive(Debug, Error)]
pub enum ProjectError {
    /// No `pact.toml` was found between `searched_from` and the filesystem root.
    #[error("no `pact.toml` found searching up from `{searched_from}`")]
    ManifestNotFound {
        /// The directory the search started from.
        searched_from: PathBuf,
    },

    /// A `pact.toml` was found but could not be parsed or validated.
    #[error(transparent)]
    ManifestError(#[from] ManifestError),

    /// The project root has no `src/` directory.
    #[error("project at `{root}` has no `src/` directory")]
    NoSourceDir {
        /// The project root that was searched.
        root: PathBuf,
    },

    /// The `src/` path exists but is a file rather than a directory.
    #[error("`{path}` exists but is not a directory")]
    SourceDirNotADirectory {
        /// The path that was expected to be a directory.
        path: PathBuf,
    },

    /// An I/O error occurred while inspecting `path`.
    #[error("I/O error at `{path}`: {source}")]
    IoError {
        /// The path being accessed when the error occurred.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// The complete set of source files for a Pact project.
#[derive(Debug, Clone)]
pub struct SourceSet {
    /// Absolute path of the project root (the directory holding `pact.toml`).
    root: PathBuf,
    /// All `.pact` files found under `<root>/src/`, sorted lexicographically.
    sources: Vec<PathBuf>,
    /// All `.pact` files found under `<root>/test/`, sorted lexicographically.
    tests: Vec<PathBuf>,
    /// The parsed `pact.toml` manifest.
    manifest: Manifest,
}

impl SourceSet {
    /// Returns the absolute, canonicalized path of the project root.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns the sorted list of `.pact` source files under `<root>/src/`.
    #[must_use]
    pub fn sources(&self) -> &[PathBuf] {
        &self.sources
    }

    /// Returns the sorted list of `.pact` test files under `<root>/test/`.
    #[must_use]
    pub fn tests(&self) -> &[PathBuf] {
        &self.tests
    }

    /// Returns the parsed `pact.toml` manifest.
    #[must_use]
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }
}

// ---------------------------------------------------------------------------
// find_project_root
// ---------------------------------------------------------------------------

/// Walk up from `start` looking for a directory that contains `pact.toml`.
///
/// The search begins at `start` itself and proceeds toward the filesystem root,
/// stopping as soon as `pact.toml` is found in the current candidate directory.
///
/// Symlinks in `start` are resolved via [`std::fs::canonicalize`] before the
/// search begins, so the returned path is always a fully resolved, absolute
/// path with no symlink components.
///
/// # Errors
///
/// - [`ProjectError::IoError`] — if `start` cannot be canonicalized (e.g., it
///   does not exist or is not accessible).
/// - [`ProjectError::ManifestNotFound`] — if `pact.toml` is not present in
///   any ancestor directory (including `start`).
///
/// # Examples
///
/// ```no_run
/// use std::path::Path;
/// use pact_compiler::project::find_project_root;
///
/// let root = find_project_root(Path::new("/home/user/my-project/src"))
///     .expect("not inside a Pact project");
/// assert!(root.join("pact.toml").exists());
/// ```
pub fn find_project_root(start: &Path) -> Result<PathBuf, ProjectError> {
    // Resolve to an absolute, symlink-free path so that ancestor traversal
    // works even when the caller passes a relative path.
    let start = start.canonicalize().map_err(|e| ProjectError::IoError {
        path: start.to_owned(),
        source: e,
    })?;

    let mut current: &Path = &start;
    loop {
        if current.join("pact.toml").is_file() {
            return Ok(current.to_owned());
        }
        match current.parent() {
            Some(parent) => current = parent,
            None => {
                return Err(ProjectError::ManifestNotFound {
                    searched_from: start,
                });
            }
        }
    }
}

// ---------------------------------------------------------------------------
// SourceSet::discover
// ---------------------------------------------------------------------------

impl SourceSet {
    /// Discover source files for the project rooted at `root`.
    ///
    /// Canonicalizes `root` at entry, then parses `<root>/pact.toml` and
    /// recursively walks `<root>/src/` and `<root>/test/` collecting every
    /// file whose name ends in `.pact`.
    ///
    /// Rules applied during the walk:
    /// - Symlinks are skipped (to prevent cycles).
    /// - Hidden entries — those whose name begins with `.` — are skipped at
    ///   every level (both directories and files).
    /// - Only files with the `.pact` extension are collected; all other files
    ///   are silently ignored.
    /// - Results within each category (`sources`, `tests`) are sorted by their
    ///   full path to give deterministic output.
    ///
    /// # Errors
    ///
    /// - [`ProjectError::IoError`] — if `root` cannot be canonicalized, or any
    ///   other I/O failure occurs during the walk.
    /// - [`ProjectError::ManifestError`] — `pact.toml` is missing or invalid.
    /// - [`ProjectError::NoSourceDir`] — `<root>/src/` does not exist.
    /// - [`ProjectError::SourceDirNotADirectory`] — `<root>/src/` exists but is
    ///   a file rather than a directory.
    ///
    /// Missing `test/` is **not** an error; `tests` will simply be empty.
    /// If `test/` exists but is not a directory it is also treated as absent.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::path::Path;
    /// use pact_compiler::project::SourceSet;
    ///
    /// let sources = SourceSet::discover(Path::new("/home/user/my-project"))
    ///     .expect("failed to collect sources");
    /// for path in sources.sources() {
    ///     println!("{}", path.display());
    /// }
    /// ```
    pub fn discover(root: &Path) -> Result<Self, ProjectError> {
        // Canonicalize first so `root` and all collected paths are consistent.
        let root = root.canonicalize().map_err(|e| ProjectError::IoError {
            path: root.to_owned(),
            source: e,
        })?;

        let manifest = Manifest::from_path(root.join("pact.toml"))?;

        let src_dir = root.join("src");
        match fs::metadata(&src_dir) {
            Ok(m) if m.is_dir() => {}
            Ok(_) => {
                return Err(ProjectError::SourceDirNotADirectory { path: src_dir });
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(ProjectError::NoSourceDir { root });
            }
            Err(e) => {
                return Err(ProjectError::IoError {
                    path: src_dir,
                    source: e,
                });
            }
        }

        let mut sources = collect_pact_files(&src_dir)?;
        sources.sort();

        let test_dir = root.join("test");
        let tests = match fs::metadata(&test_dir) {
            Ok(m) if m.is_dir() => {
                let mut t = collect_pact_files(&test_dir)?;
                t.sort();
                t
            }
            // test/ exists but is a file rather than a directory — treat as absent.
            Ok(_) => Vec::new(),
            // test/ does not exist — treat as no tests.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            // Any other I/O error (permissions, etc.) is a hard failure.
            Err(e) => {
                return Err(ProjectError::IoError {
                    path: test_dir,
                    source: e,
                });
            }
        };

        Ok(SourceSet {
            root,
            sources,
            tests,
            manifest,
        })
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Recursively collect all `.pact` files under `dir`.
///
/// Uses an iterative work queue instead of recursion to avoid stack overflow
/// on deeply nested directory trees.
///
/// Skips:
/// - symlinks
/// - hidden entries (name starts with `.`)
///
/// The returned vec is in filesystem-traversal order; callers are responsible
/// for sorting.
fn collect_pact_files(dir: &Path) -> Result<Vec<PathBuf>, ProjectError> {
    let mut results = Vec::new();
    let mut queue: VecDeque<PathBuf> = VecDeque::new();
    queue.push_back(dir.to_owned());

    while let Some(current) = queue.pop_front() {
        let entries = fs::read_dir(&current).map_err(|e| ProjectError::IoError {
            path: current.clone(),
            source: e,
        })?;

        for entry in entries {
            // The individual entry path is not available when `ReadDir` yields
            // an error for a single entry, so we report the parent directory
            // as the failing path.
            let entry = entry.map_err(|e| ProjectError::IoError {
                path: current.clone(),
                source: e,
            })?;
            let path = entry.path();

            // Skip symlinks.
            let file_type = entry.file_type().map_err(|e| ProjectError::IoError {
                path: path.clone(),
                source: e,
            })?;
            if file_type.is_symlink() {
                continue;
            }

            // Skip hidden entries — allocation-free, correct for non-UTF-8.
            let name = entry.file_name();
            if name.as_encoded_bytes().first() == Some(&b'.') {
                continue;
            }

            if file_type.is_dir() {
                queue.push_back(path);
            } else if file_type.is_file() && path.extension().is_some_and(|e| e == "pact") {
                results.push(path);
            }
        }
    }

    Ok(results)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    use tempfile::TempDir;

    use super::{SourceSet, find_project_root};
    use crate::project::ProjectError;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Minimal valid `pact.toml` content for tests that need a parseable manifest.
    const VALID_MANIFEST: &str = r#"
[package]
name    = "test-project"
version = "0.1.0"
"#;

    /// Create a file (and its parent directories) inside `base`.
    fn create_file(base: &Path, rel: &str, contents: &str) {
        let full = base.join(rel);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&full, contents).unwrap();
    }

    /// Return relative path strings for each `PathBuf`, relative to `base`,
    /// in the order they appear in the slice.
    ///
    /// Does NOT sort — the slice is expected to already be in production sort
    /// order so that tests can verify that order directly.
    fn pact_names(paths: &[PathBuf], base: &Path) -> Vec<String> {
        paths
            .iter()
            .map(|p| {
                p.strip_prefix(base)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect()
    }

    // -----------------------------------------------------------------------
    // find_project_root
    // -----------------------------------------------------------------------

    #[test]
    fn find_root_in_current_dir() {
        let tmp = TempDir::new().unwrap();
        create_file(tmp.path(), "pact.toml", VALID_MANIFEST);

        let found = find_project_root(tmp.path()).unwrap();
        assert_eq!(found, tmp.path().canonicalize().unwrap());
    }

    #[test]
    fn find_root_in_parent() {
        let tmp = TempDir::new().unwrap();
        create_file(tmp.path(), "pact.toml", VALID_MANIFEST);

        // Create a nested directory two levels deep.
        let nested = tmp.path().join("a").join("b");
        fs::create_dir_all(&nested).unwrap();

        let found = find_project_root(&nested).unwrap();
        assert_eq!(found, tmp.path().canonicalize().unwrap());
    }

    #[test]
    fn find_root_not_found() {
        let tmp = TempDir::new().unwrap();
        // No pact.toml anywhere under tmp.

        let err = find_project_root(tmp.path()).unwrap_err();
        assert!(
            matches!(err, ProjectError::ManifestNotFound { .. }),
            "expected ManifestNotFound, got {err}"
        );
    }

    #[test]
    fn find_root_nonexistent_start_path() {
        // A path that does not exist cannot be canonicalized — expect IoError.
        let err = find_project_root(Path::new("/this/path/does/not/exist/on/any/sane/system"))
            .unwrap_err();
        assert!(
            matches!(err, ProjectError::IoError { .. }),
            "expected IoError for nonexistent start path, got {err}"
        );
    }

    // -----------------------------------------------------------------------
    // SourceSet::discover
    // -----------------------------------------------------------------------

    #[test]
    fn discover_basic_project() {
        let tmp = TempDir::new().unwrap();
        create_file(tmp.path(), "pact.toml", VALID_MANIFEST);
        create_file(tmp.path(), "src/main.pact", "");
        create_file(tmp.path(), "src/lib.pact", "");

        let ss = SourceSet::discover(tmp.path()).unwrap();
        let canon = tmp.path().canonicalize().unwrap();
        let names = pact_names(ss.sources(), &canon);
        // Sorted order: lib.pact before main.pact
        assert_eq!(names, &["src/lib.pact", "src/main.pact"]);
        assert!(ss.tests().is_empty());
    }

    #[test]
    fn discover_nested_sources() {
        let tmp = TempDir::new().unwrap();
        create_file(tmp.path(), "pact.toml", VALID_MANIFEST);
        create_file(tmp.path(), "src/main.pact", "");
        create_file(tmp.path(), "src/utils/helpers.pact", "");

        let ss = SourceSet::discover(tmp.path()).unwrap();
        let canon = tmp.path().canonicalize().unwrap();
        let names = pact_names(ss.sources(), &canon);
        // Production sort order: full path sort puts "src/main.pact" before
        // "src/utils/helpers.pact" because 'm' < 'u'.
        assert_eq!(names, &["src/main.pact", "src/utils/helpers.pact"]);
    }

    #[test]
    fn discover_with_tests() {
        let tmp = TempDir::new().unwrap();
        create_file(tmp.path(), "pact.toml", VALID_MANIFEST);
        create_file(tmp.path(), "src/main.pact", "");
        create_file(tmp.path(), "test/main_test.pact", "");

        let ss = SourceSet::discover(tmp.path()).unwrap();
        let canon = tmp.path().canonicalize().unwrap();
        let test_names = pact_names(ss.tests(), &canon);
        assert_eq!(test_names, &["test/main_test.pact"]);
    }

    #[test]
    fn discover_no_test_dir() {
        let tmp = TempDir::new().unwrap();
        create_file(tmp.path(), "pact.toml", VALID_MANIFEST);
        create_file(tmp.path(), "src/main.pact", "");
        // No test/ directory.

        let ss = SourceSet::discover(tmp.path()).unwrap();
        assert!(ss.tests().is_empty(), "tests should be empty without test/");
    }

    #[test]
    fn discover_no_src_dir() {
        let tmp = TempDir::new().unwrap();
        create_file(tmp.path(), "pact.toml", VALID_MANIFEST);
        // No src/ directory.

        let err = SourceSet::discover(tmp.path()).unwrap_err();
        assert!(
            matches!(err, ProjectError::NoSourceDir { .. }),
            "expected NoSourceDir, got {err}"
        );
    }

    #[test]
    fn discover_src_is_file_not_dir() {
        let tmp = TempDir::new().unwrap();
        create_file(tmp.path(), "pact.toml", VALID_MANIFEST);
        // Create `src` as a file, not a directory.
        fs::write(tmp.path().join("src"), "I am a file").unwrap();

        let err = SourceSet::discover(tmp.path()).unwrap_err();
        assert!(
            matches!(err, ProjectError::SourceDirNotADirectory { .. }),
            "expected SourceDirNotADirectory, got {err}"
        );
    }

    #[test]
    fn discover_test_dir_is_file_treated_as_absent() {
        let tmp = TempDir::new().unwrap();
        create_file(tmp.path(), "pact.toml", VALID_MANIFEST);
        create_file(tmp.path(), "src/main.pact", "");
        // Create `test` as a file, not a directory.
        fs::write(tmp.path().join("test"), "I am a file").unwrap();

        // Should succeed, with an empty tests list.
        let ss = SourceSet::discover(tmp.path()).unwrap();
        assert!(
            ss.tests().is_empty(),
            "tests should be empty when test/ is a file"
        );
    }

    #[test]
    fn discover_skips_hidden_files() {
        let tmp = TempDir::new().unwrap();
        create_file(tmp.path(), "pact.toml", VALID_MANIFEST);
        // A hidden directory at the top of src/ — should be skipped entirely.
        create_file(tmp.path(), "src/.hidden_dir/secret.pact", "");
        // A hidden file inside a visible directory — should be skipped.
        create_file(tmp.path(), "src/.hidden.pact", "");
        // A visible file that should be collected.
        create_file(tmp.path(), "src/visible.pact", "");

        let ss = SourceSet::discover(tmp.path()).unwrap();
        let canon = tmp.path().canonicalize().unwrap();
        let names = pact_names(ss.sources(), &canon);
        assert_eq!(names, &["src/visible.pact"]);
    }

    #[test]
    fn discover_skips_non_pact_files() {
        let tmp = TempDir::new().unwrap();
        create_file(tmp.path(), "pact.toml", VALID_MANIFEST);
        create_file(tmp.path(), "src/notes.txt", "");
        create_file(tmp.path(), "src/README.md", "");
        create_file(tmp.path(), "src/main.pact", "");

        let ss = SourceSet::discover(tmp.path()).unwrap();
        let canon = tmp.path().canonicalize().unwrap();
        let names = pact_names(ss.sources(), &canon);
        assert_eq!(names, &["src/main.pact"]);
    }

    #[test]
    fn discover_empty_src_dir() {
        let tmp = TempDir::new().unwrap();
        create_file(tmp.path(), "pact.toml", VALID_MANIFEST);
        // Create src/ but leave it empty.
        fs::create_dir(tmp.path().join("src")).unwrap();

        let ss = SourceSet::discover(tmp.path()).unwrap();
        assert!(
            ss.sources().is_empty(),
            "sources should be empty for an empty src/"
        );
    }

    #[test]
    fn discover_manifest_error() {
        let tmp = TempDir::new().unwrap();
        create_file(tmp.path(), "pact.toml", "this is not valid toml ][");
        fs::create_dir(tmp.path().join("src")).unwrap();

        let err = SourceSet::discover(tmp.path()).unwrap_err();
        assert!(
            matches!(err, ProjectError::ManifestError(_)),
            "expected ManifestError, got {err}"
        );
    }

    #[test]
    fn discover_missing_pact_toml() {
        let tmp = TempDir::new().unwrap();
        // Create src/main.pact but deliberately omit pact.toml.
        create_file(tmp.path(), "src/main.pact", "");

        let err = SourceSet::discover(tmp.path()).unwrap_err();
        assert!(
            matches!(err, ProjectError::ManifestError(_)),
            "expected ManifestError when pact.toml is absent, got {err}"
        );
    }

    #[test]
    fn discover_accessors_return_consistent_data() {
        let tmp = TempDir::new().unwrap();
        create_file(tmp.path(), "pact.toml", VALID_MANIFEST);
        create_file(tmp.path(), "src/a.pact", "");
        create_file(tmp.path(), "test/b.pact", "");

        let ss = SourceSet::discover(tmp.path()).unwrap();
        let canon = tmp.path().canonicalize().unwrap();

        assert_eq!(ss.root(), canon);
        assert_eq!(ss.sources().len(), 1);
        assert_eq!(ss.tests().len(), 1);
        assert_eq!(ss.manifest().package.name, "test-project");
    }

    // -----------------------------------------------------------------------
    // Unix-specific tests
    // -----------------------------------------------------------------------

    #[cfg(unix)]
    #[test]
    fn discover_io_error_on_unreadable_subdir() {
        use std::os::unix::fs::PermissionsExt;

        // Root bypasses permission checks, so the test would produce a false
        // pass (directory still readable despite mode 0o000).  Skip it.
        // We detect root via /proc/self/status to avoid adding a libc dep.
        let uid_is_root = std::fs::read_to_string("/proc/self/status")
            .ok()
            .and_then(|s| {
                s.lines()
                    .find(|l| l.starts_with("Uid:"))
                    .and_then(|l| l.split_whitespace().nth(1))
                    .and_then(|v| v.parse::<u32>().ok())
            })
            .is_some_and(|uid| uid == 0);
        if uid_is_root {
            return;
        }

        let tmp = TempDir::new().unwrap();
        create_file(tmp.path(), "pact.toml", VALID_MANIFEST);
        // Create a visible subdirectory inside src/ …
        let secret = tmp.path().join("src").join("secret");
        fs::create_dir_all(&secret).unwrap();
        create_file(tmp.path(), "src/secret/hidden.pact", "");

        // … then remove all permissions so read_dir will fail.
        fs::set_permissions(&secret, fs::Permissions::from_mode(0o000)).unwrap();

        let result = SourceSet::discover(tmp.path());

        // Restore permissions before any assertion so TempDir can clean up.
        fs::set_permissions(&secret, fs::Permissions::from_mode(0o755)).unwrap();

        assert!(
            matches!(result, Err(ProjectError::IoError { .. })),
            "expected IoError for unreadable directory, got {result:?}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn discover_does_not_collect_symlinks() {
        use std::os::unix::fs::symlink;

        let tmp = TempDir::new().unwrap();
        create_file(tmp.path(), "pact.toml", VALID_MANIFEST);
        create_file(tmp.path(), "src/real.pact", "");

        // Create a symlink inside src/ pointing at the real file.
        let link = tmp.path().join("src").join("link.pact");
        symlink(tmp.path().join("src").join("real.pact"), &link).unwrap();

        let ss = SourceSet::discover(tmp.path()).unwrap();
        let canon = tmp.path().canonicalize().unwrap();
        let names = pact_names(ss.sources(), &canon);
        // Only the real file, not the symlink.
        assert_eq!(names, &["src/real.pact"]);
    }
}
