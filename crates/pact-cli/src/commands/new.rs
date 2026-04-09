//! Handler for `pact new`.
//!
//! Scaffolds a new Pact project directory from one of the built-in templates.
//! Template files live under `templates/` at the crate root and are embedded
//! at compile time via [`include_str!`].  The placeholder `{{name}}` in every
//! template file is replaced with the project name at generation time.
//!
//! # Examples
//!
//! ```text
//! pact new myapp               # bin template (default)
//! pact new mylib --template lib
//! pact new my-agent --template agent
//! pact new myapp --force       # overwrite existing directory
//! ```

use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::path::Path;

use anyhow::{Context as _, Result, bail};
use clap::{Parser, ValueEnum};

use crate::GlobalOpts;

// ---------------------------------------------------------------------------
// Template text — embedded at compile time
// ---------------------------------------------------------------------------

/// Raw text for `bin/pact.toml`.
const BIN_MANIFEST: &str = include_str!("../../templates/bin/pact.toml");
/// Raw text for `bin/src/main.pact`.
const BIN_MAIN: &str = include_str!("../../templates/bin/src/main.pact");
/// Raw text for `bin/test/main_test.pact`.
const BIN_MAIN_TEST: &str = include_str!("../../templates/bin/test/main_test.pact");
/// Raw text for `bin/.gitignore`.
const BIN_GITIGNORE: &str = include_str!("../../templates/bin/.gitignore");

/// Raw text for `lib/pact.toml`.
const LIB_MANIFEST: &str = include_str!("../../templates/lib/pact.toml");
/// Raw text for `lib/src/lib.pact`.
const LIB_SRC: &str = include_str!("../../templates/lib/src/lib.pact");
/// Raw text for `lib/test/lib_test.pact`.
const LIB_TEST: &str = include_str!("../../templates/lib/test/lib_test.pact");
/// Raw text for `lib/.gitignore`.
const LIB_GITIGNORE: &str = include_str!("../../templates/lib/.gitignore");

/// Raw text for `agent/pact.toml`.
const AGENT_MANIFEST: &str = include_str!("../../templates/agent/pact.toml");
/// Raw text for `agent/src/main.pact`.
const AGENT_MAIN: &str = include_str!("../../templates/agent/src/main.pact");
/// Raw text for `agent/test/main_test.pact`.
const AGENT_MAIN_TEST: &str = include_str!("../../templates/agent/test/main_test.pact");
/// Raw text for `agent/.gitignore`.
const AGENT_GITIGNORE: &str = include_str!("../../templates/agent/.gitignore");

// ---------------------------------------------------------------------------
// Static template tables
// ---------------------------------------------------------------------------

/// A single file entry in a template: `(relative_path, raw_content)`.
///
/// `relative_path` is relative to the project root.  `raw_content` may
/// contain `{{name}}` placeholders that are replaced with the project name at
/// generation time.
type TemplateEntry = (&'static str, &'static str);

/// Files generated for the `bin` template.
static BIN_FILES: &[TemplateEntry] = &[
    ("pact.toml", BIN_MANIFEST),
    ("src/main.pact", BIN_MAIN),
    ("test/main_test.pact", BIN_MAIN_TEST),
    (".gitignore", BIN_GITIGNORE),
];

/// Files generated for the `lib` template.
static LIB_FILES: &[TemplateEntry] = &[
    ("pact.toml", LIB_MANIFEST),
    ("src/lib.pact", LIB_SRC),
    ("test/lib_test.pact", LIB_TEST),
    (".gitignore", LIB_GITIGNORE),
];

/// Files generated for the `agent` template.
static AGENT_FILES: &[TemplateEntry] = &[
    ("pact.toml", AGENT_MANIFEST),
    ("src/main.pact", AGENT_MAIN),
    ("test/main_test.pact", AGENT_MAIN_TEST),
    (".gitignore", AGENT_GITIGNORE),
];

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Project template variants for `pact new`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub(crate) enum Template {
    /// A binary (executable) project.
    #[default]
    Bin,
    /// A library project.
    Lib,
    /// A sandboxed agent project using `AgentIO`.
    Agent,
}

impl fmt::Display for Template {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bin => f.write_str("bin"),
            Self::Lib => f.write_str("lib"),
            Self::Agent => f.write_str("agent"),
        }
    }
}

/// Arguments for `pact new`.
#[derive(Clone, Debug, Parser)]
pub(crate) struct NewCmd {
    /// The name of the new project (also used as the directory name).
    #[arg(value_name = "NAME")]
    pub(crate) name: String,

    /// Project template to use.
    ///
    /// Accepted values: `bin` (default), `lib`, `agent`.
    #[arg(long, value_name = "TEMPLATE", default_value = "bin")]
    pub(crate) template: Template,

    /// Overwrite files in an existing non-empty directory.
    ///
    /// Without this flag, `pact new` exits with an error if the target
    /// directory already exists and contains files.
    #[arg(long)]
    pub(crate) force: bool,
}

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

/// Validate that `name` is a legal Pact project name.
///
/// Rules:
/// - Non-empty.
/// - No path separators (`/` or `\`).
/// - Not `.` or `..`.
/// - Starts with `[a-z]`, contains only `[a-z0-9-]`.
/// - No consecutive, leading, or trailing hyphens.
///
/// # Errors
///
/// Returns an error describing the first violated rule.
fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("invalid project name '': name must not be empty");
    }

    if name.contains('/') || name.contains('\\') {
        bail!(
            "invalid project name '{name}': must be lowercase, start with a letter, \
             contain only letters, digits, and hyphens"
        );
    }

    if name == "." || name == ".." {
        bail!(
            "invalid project name '{name}': must be lowercase, start with a letter, \
             contain only letters, digits, and hyphens"
        );
    }

    // Must consist solely of [a-z0-9-].
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        bail!(
            "invalid project name '{name}': must be lowercase, start with a letter, \
             contain only letters, digits, and hyphens"
        );
    }

    // Must start with a letter.
    if !name.starts_with(|c: char| c.is_ascii_lowercase()) {
        bail!(
            "invalid project name '{name}': must be lowercase, start with a letter, \
             contain only letters, digits, and hyphens"
        );
    }

    // Must not end with a hyphen.
    if name.ends_with('-') {
        bail!(
            "invalid project name '{name}': must be lowercase, start with a letter, \
             contain only letters, digits, and hyphens"
        );
    }

    // Must not contain consecutive hyphens.
    if name.contains("--") {
        bail!(
            "invalid project name '{name}': must be lowercase, start with a letter, \
             contain only letters, digits, and hyphens"
        );
    }

    Ok(())
}

/// Return the slice of [`TemplateEntry`] items for the given [`Template`].
fn template_files(template: Template) -> &'static [TemplateEntry] {
    match template {
        Template::Bin => BIN_FILES,
        Template::Lib => LIB_FILES,
        Template::Agent => AGENT_FILES,
    }
}

/// Execute `pact new`.
///
/// Scaffolds a new Pact project in a directory named `<name>` relative to the
/// current working directory.  The `--template` flag selects which template to
/// use (`bin` by default).  Pass `--force` to allow generation into an existing
/// non-empty directory.
///
/// # Errors
///
/// Returns an error if:
/// - The project name is invalid (see [`validate_name`]).
/// - The target directory is non-empty and `--force` is not set.
/// - Any file or directory cannot be created.
pub(crate) fn run_new(cmd: &NewCmd, global: &GlobalOpts) -> Result<()> {
    // Validate name before any filesystem work.
    validate_name(&cmd.name)?;

    let project_dir = Path::new(&cmd.name);

    // Guard: refuse to clobber a non-empty directory without --force.
    //
    // NOTE: There is an inherent TOCTOU race here — between this check and the
    // subsequent writes another process could create or remove files in the
    // directory.  This is a best-effort guard rather than a security boundary;
    // the worst outcome is an unexpected overwrite or a spurious error, both of
    // which are recoverable by the user.
    if project_dir.is_dir() {
        let existing: Vec<String> = project_dir
            .read_dir()
            .with_context(|| format!("could not read directory `{}`", project_dir.display()))?
            .filter_map(std::result::Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();

        let is_empty = existing.is_empty();

        if !is_empty && !cmd.force {
            bail!(
                "directory `{}` already exists and is non-empty; \
                 pass --force to overwrite",
                project_dir.display()
            );
        }

        // Warn about files that exist but are not part of this template.
        if cmd.force && !is_empty && !global.quiet {
            let template_names: BTreeSet<&str> = template_files(cmd.template)
                .iter()
                .filter_map(|(p, _)| p.split('/').next())
                .collect();
            for name in &existing {
                if !template_names.contains(name.as_str()) {
                    eprintln!(
                        "warning: existing file '{name}' not part of template, leaving in place"
                    );
                }
            }
        }
    }

    // Collect unique parent directories and create them up-front.
    let parent_dirs: BTreeSet<&Path> = template_files(cmd.template)
        .iter()
        .filter_map(|(rel, _)| Path::new(rel).parent())
        .filter(|p| !p.as_os_str().is_empty())
        .collect();

    for rel_parent in parent_dirs {
        let abs_parent = project_dir.join(rel_parent);
        fs::create_dir_all(&abs_parent)
            .with_context(|| format!("could not create directory `{}`", abs_parent.display()))?;
    }

    // Generate each template file with name substitution.
    for (relative_path, raw_content) in template_files(cmd.template) {
        let dest = project_dir.join(relative_path);
        let content = raw_content.replace("{{name}}", &cmd.name);
        fs::write(&dest, content)
            .with_context(|| format!("could not write file `{}`", dest.display()))?;
    }

    if !global.quiet {
        println!("Created {} project '{}'", cmd.template, cmd.name);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use clap::Parser as _;

    use super::*;
    use crate::Cli;

    /// Parse a `Cli` from argument slices (prefixed with the binary name).
    fn parse_cli(args: &[&str]) -> Result<Cli, clap::Error> {
        let full: Vec<&str> = std::iter::once("pact")
            .chain(args.iter().copied())
            .collect();
        Cli::try_parse_from(full)
    }

    // --- --force flag -------------------------------------------------------

    /// `--force` defaults to `false` when omitted.
    #[test]
    fn force_flag_defaults_to_false() {
        let cli = parse_cli(&["new", "myapp"]).unwrap();
        if let crate::Commands::New(cmd) = &cli.command {
            assert!(!cmd.force, "--force should default to false");
        } else {
            panic!("expected New command");
        }
    }

    /// `--force` is `true` when supplied.
    #[test]
    fn force_flag_parses() {
        let cli = parse_cli(&["new", "myapp", "--force"]).unwrap();
        if let crate::Commands::New(cmd) = &cli.command {
            assert!(cmd.force, "--force should be true when passed");
        } else {
            panic!("expected New command");
        }
    }

    // --- template tables ----------------------------------------------------

    /// The `bin` template exposes four file entries (including .gitignore).
    #[test]
    fn bin_template_has_four_files() {
        assert_eq!(template_files(Template::Bin).len(), 4);
    }

    /// The `lib` template exposes four file entries (including .gitignore).
    #[test]
    fn lib_template_has_four_files() {
        assert_eq!(template_files(Template::Lib).len(), 4);
    }

    /// The `agent` template exposes four file entries (including .gitignore).
    #[test]
    fn agent_template_has_four_files() {
        assert_eq!(template_files(Template::Agent).len(), 4);
    }

    /// The bin manifest template contains the `{{name}}` placeholder.
    #[test]
    fn bin_manifest_has_name_placeholder() {
        assert!(
            BIN_MANIFEST.contains("{{name}}"),
            "bin pact.toml template missing {{{{name}}}} placeholder"
        );
    }

    /// The lib source template contains the `{{name}}` placeholder.
    #[test]
    fn lib_src_has_name_placeholder() {
        assert!(
            LIB_SRC.contains("{{name}}"),
            "lib src template missing {{{{name}}}} placeholder"
        );
    }

    /// The agent manifest template contains the `{{name}}` placeholder.
    #[test]
    fn agent_manifest_has_name_placeholder() {
        assert!(
            AGENT_MANIFEST.contains("{{name}}"),
            "agent pact.toml template missing {{{{name}}}} placeholder"
        );
    }

    // --- name substitution --------------------------------------------------

    /// `{{name}}` is replaced in all occurrences within a template string.
    #[test]
    fn placeholder_replacement_replaces_all() {
        let template = "name = \"{{name}}\"\n-- {{name}} library\n";
        let result = template.replace("{{name}}", "mypkg");
        assert_eq!(result, "name = \"mypkg\"\n-- mypkg library\n");
        assert!(
            !result.contains("{{name}}"),
            "replacement left a residual placeholder"
        );
    }

    // --- Template::Display --------------------------------------------------

    /// `Template::Bin` displays as "bin".
    #[test]
    fn template_display_bin() {
        assert_eq!(Template::Bin.to_string(), "bin");
    }

    /// `Template::Lib` displays as "lib".
    #[test]
    fn template_display_lib() {
        assert_eq!(Template::Lib.to_string(), "lib");
    }

    /// `Template::Agent` displays as "agent".
    #[test]
    fn template_display_agent() {
        assert_eq!(Template::Agent.to_string(), "agent");
    }

    // --- validate_name ------------------------------------------------------

    /// Empty name is rejected.
    #[test]
    fn validate_name_rejects_empty() {
        assert!(validate_name("").is_err());
    }

    /// Name with a forward slash is rejected.
    #[test]
    fn validate_name_rejects_path_traversal_forward_slash() {
        assert!(validate_name("../evil").is_err());
    }

    /// Name with a backslash is rejected.
    #[test]
    fn validate_name_rejects_path_traversal_backslash() {
        assert!(validate_name("..\\evil").is_err());
    }

    /// Absolute path is rejected.
    #[test]
    fn validate_name_rejects_absolute_path() {
        assert!(validate_name("/tmp/bad").is_err());
    }

    /// Name containing spaces is rejected.
    #[test]
    fn validate_name_rejects_spaces() {
        assert!(validate_name("my app").is_err());
    }

    /// Uppercase letters are rejected.
    #[test]
    fn validate_name_rejects_uppercase() {
        assert!(validate_name("MyApp").is_err());
    }

    /// `.` is rejected.
    #[test]
    fn validate_name_rejects_single_dot() {
        assert!(validate_name(".").is_err());
    }

    /// `..` is rejected.
    #[test]
    fn validate_name_rejects_double_dot() {
        assert!(validate_name("..").is_err());
    }

    /// A single-character valid name is accepted.
    #[test]
    fn validate_name_accepts_single_char() {
        assert!(validate_name("a").is_ok());
    }

    /// A hyphenated name is accepted.
    #[test]
    fn validate_name_accepts_hyphenated() {
        assert!(validate_name("my-app").is_ok());
    }

    /// Leading hyphen is rejected.
    #[test]
    fn validate_name_rejects_leading_hyphen() {
        assert!(validate_name("-myapp").is_err());
    }

    /// Trailing hyphen is rejected.
    #[test]
    fn validate_name_rejects_trailing_hyphen() {
        assert!(validate_name("myapp-").is_err());
    }

    /// Consecutive hyphens are rejected.
    #[test]
    fn validate_name_rejects_consecutive_hyphens() {
        assert!(validate_name("my--app").is_err());
    }

    /// Names starting with a digit are rejected.
    #[test]
    fn validate_name_rejects_digit_prefix() {
        assert!(validate_name("1abc").is_err());
    }
}
