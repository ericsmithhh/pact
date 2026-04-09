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
//! pact new myapp --force       # overwrite existing directory
//! ```

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

/// Raw text for `lib/pact.toml`.
const LIB_MANIFEST: &str = include_str!("../../templates/lib/pact.toml");
/// Raw text for `lib/src/lib.pact`.
const LIB_SRC: &str = include_str!("../../templates/lib/src/lib.pact");
/// Raw text for `lib/test/lib_test.pact`.
const LIB_TEST: &str = include_str!("../../templates/lib/test/lib_test.pact");

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
];

/// Files generated for the `lib` template.
static LIB_FILES: &[TemplateEntry] = &[
    ("pact.toml", LIB_MANIFEST),
    ("src/lib.pact", LIB_SRC),
    ("test/lib_test.pact", LIB_TEST),
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
}

/// Arguments for `pact new`.
#[derive(Clone, Debug, Parser)]
pub(crate) struct NewCmd {
    /// The name of the new project (also used as the directory name).
    #[arg(value_name = "NAME")]
    pub(crate) name: String,

    /// Project template to use.
    ///
    /// Accepted values: `bin` (default), `lib`.
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

/// Return the slice of [`TemplateEntry`] items for the given [`Template`].
fn template_files(template: Template) -> &'static [TemplateEntry] {
    match template {
        Template::Bin => BIN_FILES,
        Template::Lib => LIB_FILES,
    }
}

/// Write `content` to `path`, creating all intermediate directories first.
fn write_file(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("could not create directory `{}`", parent.display()))?;
    }
    fs::write(path, content).with_context(|| format!("could not write file `{}`", path.display()))
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
/// - The target directory is non-empty and `--force` is not set.
/// - Any file or directory cannot be created.
pub(crate) fn run_new(cmd: &NewCmd, _global: &GlobalOpts) -> Result<()> {
    let project_dir = Path::new(&cmd.name);

    // Guard: refuse to clobber a non-empty directory without --force.
    if project_dir.is_dir() {
        let is_empty = project_dir
            .read_dir()
            .with_context(|| format!("could not read directory `{}`", project_dir.display()))?
            .next()
            .is_none();

        if !is_empty && !cmd.force {
            bail!(
                "directory `{}` already exists and is non-empty; \
                 pass --force to overwrite",
                project_dir.display()
            );
        }
    }

    // Generate each template file with name substitution.
    for (relative_path, raw_content) in template_files(cmd.template) {
        let dest = project_dir.join(relative_path);
        let content = raw_content.replace("{{name}}", &cmd.name);
        write_file(&dest, &content)?;
    }

    let template_label = match cmd.template {
        Template::Bin => "bin",
        Template::Lib => "lib",
    };

    println!("Created {template_label} project '{}'", cmd.name);

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

    /// The `bin` template exposes three file entries.
    #[test]
    fn bin_template_has_three_files() {
        assert_eq!(template_files(Template::Bin).len(), 3);
    }

    /// The `lib` template exposes three file entries.
    #[test]
    fn lib_template_has_three_files() {
        assert_eq!(template_files(Template::Lib).len(), 3);
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
}
