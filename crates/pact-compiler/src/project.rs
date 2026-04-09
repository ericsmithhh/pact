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
//! println!("found {} source files", sources.sources.len());
//! ```

use std::{
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

    /// An I/O error occurred while inspecting `path`.
    #[error("I/O error at `{0}`: {1}")]
    IoError(PathBuf, #[source] std::io::Error),
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// The complete set of source files for a Pact project.
#[derive(Debug, Clone)]
pub struct SourceSet {
    /// Absolute path of the project root (the directory holding `pact.toml`).
    pub root: PathBuf,
    /// All `.pact` files found under `<root>/src/`, sorted lexicographically.
    pub sources: Vec<PathBuf>,
    /// All `.pact` files found under `<root>/test/`, sorted lexicographically.
    pub tests: Vec<PathBuf>,
    /// The parsed `pact.toml` manifest.
    pub manifest: Manifest,
}

// ---------------------------------------------------------------------------
// find_project_root
// ---------------------------------------------------------------------------

/// Walk up from `start` looking for a directory that contains `pact.toml`.
///
/// The search begins at `start` itself and proceeds toward the filesystem root,
/// stopping as soon as `pact.toml` is found in the current candidate directory.
///
/// # Errors
///
/// Returns [`ProjectError::ManifestNotFound`] if `pact.toml` is not present in
/// any ancestor directory (including `start`).
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
    // Resolve to an absolute path so that ancestor traversal works even when
    // the caller passes a relative path.
    let start = start
        .canonicalize()
        .map_err(|e| ProjectError::IoError(start.to_owned(), e))?;

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
    /// Parses `<root>/pact.toml`, then recursively walks `<root>/src/` and
    /// `<root>/test/` collecting every file whose name ends in `.pact`.
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
    /// - [`ProjectError::ManifestError`] — `pact.toml` is missing or invalid.
    /// - [`ProjectError::NoSourceDir`] — `<root>/src/` does not exist.
    /// - [`ProjectError::IoError`] — any other I/O failure during the walk.
    ///
    /// Missing `test/` is **not** an error; `tests` will simply be empty.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::path::Path;
    /// use pact_compiler::project::SourceSet;
    ///
    /// let sources = SourceSet::discover(Path::new("/home/user/my-project"))
    ///     .expect("failed to collect sources");
    /// for path in &sources.sources {
    ///     println!("{}", path.display());
    /// }
    /// ```
    pub fn discover(root: &Path) -> Result<Self, ProjectError> {
        let manifest = Manifest::from_path(root.join("pact.toml"))?;

        let src_dir = root.join("src");
        if !src_dir.is_dir() {
            return Err(ProjectError::NoSourceDir {
                root: root.to_owned(),
            });
        }

        let mut sources = collect_pact_files(&src_dir)?;
        sources.sort();

        let test_dir = root.join("test");
        let mut tests = if test_dir.is_dir() {
            let mut t = collect_pact_files(&test_dir)?;
            t.sort();
            t
        } else {
            Vec::new()
        };
        tests.sort();

        Ok(SourceSet {
            root: root.to_owned(),
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
/// Skips:
/// - symlinks
/// - hidden entries (name starts with `.`)
///
/// The returned vec is in filesystem-traversal order; callers are responsible
/// for sorting.
fn collect_pact_files(dir: &Path) -> Result<Vec<PathBuf>, ProjectError> {
    let mut results = Vec::new();
    collect_recursive(dir, &mut results)?;
    Ok(results)
}

fn collect_recursive(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), ProjectError> {
    let entries = fs::read_dir(dir).map_err(|e| ProjectError::IoError(dir.to_owned(), e))?;

    for entry in entries {
        let entry = entry.map_err(|e| ProjectError::IoError(dir.to_owned(), e))?;
        let path = entry.path();

        // Skip symlinks.
        let file_type = entry
            .file_type()
            .map_err(|e| ProjectError::IoError(path.clone(), e))?;
        if file_type.is_symlink() {
            continue;
        }

        // Skip hidden entries.
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with('.') {
            continue;
        }

        if file_type.is_dir() {
            collect_recursive(&path, out)?;
        } else if file_type.is_file() && path.extension().is_some_and(|e| e == "pact") {
            out.push(path);
        }
    }

    Ok(())
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
        let names = pact_names(&ss.sources, tmp.path());
        // Sorted order: lib.pact before main.pact
        assert_eq!(names, &["src/lib.pact", "src/main.pact"]);
        assert!(ss.tests.is_empty());
    }

    #[test]
    fn discover_nested_sources() {
        let tmp = TempDir::new().unwrap();
        create_file(tmp.path(), "pact.toml", VALID_MANIFEST);
        create_file(tmp.path(), "src/main.pact", "");
        create_file(tmp.path(), "src/utils/helpers.pact", "");

        let ss = SourceSet::discover(tmp.path()).unwrap();
        let names = pact_names(&ss.sources, tmp.path());
        assert_eq!(names.len(), 2, "expected 2 sources, got: {names:?}");
        assert!(names.iter().any(|n| n == "src/main.pact"));
        assert!(names.iter().any(|n| n == "src/utils/helpers.pact"));
    }

    #[test]
    fn discover_with_tests() {
        let tmp = TempDir::new().unwrap();
        create_file(tmp.path(), "pact.toml", VALID_MANIFEST);
        create_file(tmp.path(), "src/main.pact", "");
        create_file(tmp.path(), "test/main_test.pact", "");

        let ss = SourceSet::discover(tmp.path()).unwrap();
        let test_names = pact_names(&ss.tests, tmp.path());
        assert_eq!(test_names, &["test/main_test.pact"]);
    }

    #[test]
    fn discover_no_test_dir() {
        let tmp = TempDir::new().unwrap();
        create_file(tmp.path(), "pact.toml", VALID_MANIFEST);
        create_file(tmp.path(), "src/main.pact", "");
        // No test/ directory.

        let ss = SourceSet::discover(tmp.path()).unwrap();
        assert!(ss.tests.is_empty(), "tests should be empty without test/");
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
        let names = pact_names(&ss.sources, tmp.path());
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
        let names = pact_names(&ss.sources, tmp.path());
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
            ss.sources.is_empty(),
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

    // -----------------------------------------------------------------------
    // Utility
    // -----------------------------------------------------------------------

    /// Return relative path strings for each `PathBuf`, relative to `base`,
    /// in the order they appear in the slice.
    fn pact_names(paths: &[PathBuf], base: &Path) -> Vec<String> {
        let mut names: Vec<String> = paths
            .iter()
            .map(|p| {
                p.strip_prefix(base)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect();
        names.sort();
        names
    }
}
