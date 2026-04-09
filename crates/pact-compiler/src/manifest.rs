//! Parsing and validation of `pact.toml` project manifests.
//!
//! A `pact.toml` file describes a Pact package: its identity, external
//! dependencies, pact capability requirements, and resource-usage seal policy.
//!
//! # Example
//!
//! ```toml
//! [package]
//! name        = "weather-agent"
//! version     = "0.1.0"
//! authors     = ["you@example.com"]
//! license     = "MIT"
//! description = "A weather-aware scheduling agent"
//!
//! [dependencies]
//! http-client = "1.2.0"
//!
//! [pacts]
//! required = ["Http", "Console"]
//! optional = ["FileSystem"]
//!
//! [seal]
//! max_memory    = "256mb"
//! max_duration  = "30s"
//! allowed_hosts = ["api.openweathermap.org"]
//! ```
//!
//! Use the [`std::str::FromStr`] impl (`toml_str.parse::<Manifest>()`) or
//! [`Manifest::from_path`] to obtain a parsed, validated [`Manifest`].

use std::{collections::BTreeMap, path::Path, path::PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A fully-parsed and validated `pact.toml` manifest.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Manifest {
    /// Package identity metadata.
    pub package: PackageInfo,
    /// External package dependencies, keyed by name with a version requirement.
    pub dependencies: BTreeMap<String, String>,
    /// Pact capability requirements for this package.
    pub pacts: PactRequirements,
    /// Optional resource-usage constraints (the "seal").
    pub seal: Option<SealPolicy>,
}

/// Identity and metadata for the package.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PackageInfo {
    /// Package name: lowercase ASCII letters, digits, and hyphens only.
    pub name: String,
    /// Package version in `MAJOR.MINOR.PATCH` semver form.
    pub version: String,
    /// Optional list of author contact strings.
    pub authors: Vec<String>,
    /// Optional SPDX license expression.
    pub license: Option<String>,
    /// Optional human-readable description.
    pub description: Option<String>,
}

/// Pact capability requirements declared by the package.
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct PactRequirements {
    /// Capabilities that must be provided at runtime.
    pub required: Vec<String>,
    /// Capabilities that enhance functionality when available.
    pub optional: Vec<String>,
}

/// Resource-usage constraints imposed on the running package.
///
/// # Validation note
///
/// The individual field values (e.g. `max_memory = "256mb"`, `max_duration =
/// "30s"`) are stored as raw strings at parse time.  Their format is validated
/// at **seal-enforcement time** (when the runtime or compiler applies the
/// policy), not during manifest parsing.  This keeps the manifest layer
/// decoupled from the evolving seal DSL and allows new resource kinds to be
/// introduced without breaking existing manifests.
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct SealPolicy {
    /// Maximum memory allocation, e.g. `"256mb"`.
    ///
    /// The value format is validated at seal-enforcement time, not at manifest
    /// parse time.
    pub max_memory: Option<String>,
    /// Maximum wall-clock duration, e.g. `"30s"`.
    ///
    /// The value format is validated at seal-enforcement time, not at manifest
    /// parse time.
    pub max_duration: Option<String>,
    /// Hosts the package is permitted to contact.
    pub allowed_hosts: Vec<String>,
}

/// Errors that can arise while loading or validating a manifest.
#[derive(Debug, Error)]
pub enum ManifestError {
    /// The TOML text could not be parsed.
    #[error("manifest parse error: {0}")]
    ParseError(#[from] toml::de::Error),

    /// A required field was absent from the manifest.
    ///
    /// The inner string names the field path (e.g. `"package.name"`).
    #[error("missing required field `{0}`")]
    MissingField(String),

    /// A field was present but its value is invalid.
    ///
    /// `field` is the field path (e.g. `"package.name"`); `reason` explains
    /// the constraint that was violated.
    #[error("invalid value for field `{field}`: {reason}")]
    InvalidField {
        /// The dotted field path, e.g. `"package.name"`.
        field: String,
        /// A human-readable description of why the value was rejected.
        reason: String,
    },

    /// The file could not be read from disk.
    ///
    /// Carries the path that was attempted so callers can surface it in
    /// diagnostic messages.
    #[error("could not read manifest file `{0}`: {1}")]
    IoError(PathBuf, std::io::Error),
}

// ---------------------------------------------------------------------------
// Manifest entry points
// ---------------------------------------------------------------------------

impl std::str::FromStr for Manifest {
    type Err = ManifestError;

    /// Parse and validate a manifest from a TOML string.
    ///
    /// # Errors
    ///
    /// Returns [`ManifestError::ParseError`] if the text is not valid TOML,
    /// [`ManifestError::MissingField`] if a required field is absent,
    /// [`ManifestError::InvalidField`] if a field value violates its
    /// naming or versioning rules.
    fn from_str(toml_str: &str) -> Result<Self, Self::Err> {
        let raw: RawManifest = toml::from_str(toml_str)?;
        raw.validate()
    }
}

impl Manifest {
    /// Read the file at `path` and parse it as a TOML manifest.
    ///
    /// # Errors
    ///
    /// Returns [`ManifestError::IoError`] (carrying the attempted path) if the
    /// file cannot be read, and all errors from [`std::str::FromStr`] for
    /// parsing/validation failures.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use pact_compiler::manifest::Manifest;
    ///
    /// let manifest = Manifest::from_path("pact.toml").expect("failed to load manifest");
    /// println!("package: {}", manifest.package.name);
    /// ```
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, ManifestError> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path)
            .map_err(|e| ManifestError::IoError(path.to_owned(), e))?;
        text.parse()
    }
}

// ---------------------------------------------------------------------------
// Raw (deserialisation) layer — not part of the public API
// ---------------------------------------------------------------------------

// NOTE: no deny_unknown_fields — forward compatibility with future manifest sections
/// Mirrors the TOML shape exactly; validation happens in `validate()`.
///
/// `deny_unknown_fields` is intentionally **not** set so that manifests
/// written for future versions of Pact (which may add new top-level sections)
/// remain loadable by older toolchains without errors.
#[derive(Debug, Deserialize)]
struct RawManifest {
    package: Option<RawPackage>,
    #[serde(default)]
    dependencies: BTreeMap<String, String>,
    #[serde(default)]
    pacts: RawPacts,
    seal: Option<RawSeal>,
}

#[derive(Debug, Deserialize)]
struct RawPackage {
    name: Option<String>,
    version: Option<String>,
    #[serde(default)]
    authors: Vec<String>,
    license: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct RawPacts {
    #[serde(default)]
    required: Vec<String>,
    #[serde(default)]
    optional: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RawSeal {
    max_memory: Option<String>,
    max_duration: Option<String>,
    #[serde(default)]
    allowed_hosts: Vec<String>,
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

impl RawManifest {
    fn validate(self) -> Result<Manifest, ManifestError> {
        let raw_pkg = self
            .package
            .ok_or_else(|| ManifestError::MissingField("package".into()))?;

        let name = raw_pkg
            .name
            .ok_or_else(|| ManifestError::MissingField("package.name".into()))?;

        validate_package_name(&name)?;

        let version = raw_pkg
            .version
            .ok_or_else(|| ManifestError::MissingField("package.version".into()))?;

        validate_version(&version)?;

        let package = PackageInfo {
            name,
            version,
            authors: raw_pkg.authors,
            license: raw_pkg.license,
            description: raw_pkg.description,
        };

        for (dep_name, dep_version) in &self.dependencies {
            validate_version_requirement(dep_name, dep_version)?;
        }

        let mut required_pacts = Vec::with_capacity(self.pacts.required.len());
        for pact_name in self.pacts.required {
            validate_pact_name(&pact_name, "pacts.required")?;
            required_pacts.push(pact_name);
        }

        let mut optional_pacts = Vec::with_capacity(self.pacts.optional.len());
        for pact_name in self.pacts.optional {
            validate_pact_name(&pact_name, "pacts.optional")?;
            optional_pacts.push(pact_name);
        }

        let pacts = PactRequirements {
            required: required_pacts,
            optional: optional_pacts,
        };

        let seal = self.seal.map(|s| SealPolicy {
            max_memory: s.max_memory,
            max_duration: s.max_duration,
            allowed_hosts: s.allowed_hosts,
        });

        Ok(Manifest {
            package,
            dependencies: self.dependencies,
            pacts,
            seal,
        })
    }
}

/// A valid package name:
/// - contains only lowercase ASCII letters, ASCII digits, and hyphens
/// - starts and ends with `[a-z0-9]` (not a hyphen)
/// - has no consecutive hyphens (`--`)
/// - contains at least one ASCII letter (not all digits)
fn validate_package_name(name: &str) -> Result<(), ManifestError> {
    const FIELD: &str = "package.name";
    const REASON: &str = "must contain only lowercase ASCII letters, digits, and hyphens; \
        must start and end with a letter or digit; must not contain consecutive hyphens; \
        must contain at least one letter";

    if name.is_empty() {
        return Err(ManifestError::MissingField(FIELD.into()));
    }

    // All characters must be lowercase ASCII letters, digits, or hyphens.
    let all_valid_chars = name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !all_valid_chars {
        return Err(ManifestError::InvalidField {
            field: FIELD.into(),
            reason: REASON.into(),
        });
    }

    // Must start and end with [a-z0-9].
    let starts_ok = name
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    let ends_ok = name
        .chars()
        .next_back()
        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    if !starts_ok || !ends_ok {
        return Err(ManifestError::InvalidField {
            field: FIELD.into(),
            reason: REASON.into(),
        });
    }

    // No consecutive hyphens.
    if name.contains("--") {
        return Err(ManifestError::InvalidField {
            field: FIELD.into(),
            reason: REASON.into(),
        });
    }

    // Must contain at least one ASCII letter (pure digit names like "123" are rejected).
    let has_letter = name.chars().any(|c| c.is_ascii_lowercase());
    if !has_letter {
        return Err(ManifestError::InvalidField {
            field: FIELD.into(),
            reason: REASON.into(),
        });
    }

    Ok(())
}

/// A valid version string matches the pattern `DIGITS.DIGITS.DIGITS` with
/// optional pre-release / build-metadata suffixes.  We do a lightweight
/// structural check rather than full semver parsing.
fn validate_version(version: &str) -> Result<(), ManifestError> {
    // Accept `X.Y.Z` or `X.Y.Z-pre` or `X.Y.Z+meta` or both.
    // Strategy: strip optional `-…` and `+…` suffixes, then check `X.Y.Z`.
    // A trailing `+` or `-` with an empty suffix is invalid.
    let core = if let Some((core, meta)) = version.split_once('+') {
        if meta.is_empty() {
            return Err(ManifestError::InvalidField {
                field: "package.version".into(),
                reason: "trailing `+` must be followed by non-empty build metadata".into(),
            });
        }
        core
    } else {
        version
    };
    let core = if let Some((core, pre)) = core.split_once('-') {
        if pre.is_empty() {
            return Err(ManifestError::InvalidField {
                field: "package.version".into(),
                reason: "trailing `-` must be followed by non-empty pre-release identifier".into(),
            });
        }
        core
    } else {
        core
    };

    let parts: Vec<&str> = core.split('.').collect();
    let ok = parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()));
    if ok {
        Ok(())
    } else {
        Err(ManifestError::InvalidField {
            field: "package.version".into(),
            reason: "must be valid semver X.Y.Z (optionally with pre-release or build metadata)"
                .into(),
        })
    }
}

/// Validates a dependency version requirement string.
///
/// Accepts:
/// - Exact semver: `"1.0.0"`, `"1.2.3-alpha"`
/// - Caret requirements: `"^1.0"`, `"^1.0.0"`
/// - Tilde requirements: `"~1.2"`, `"~1.2.3"`
/// - Comparison: `">=1.0.0"`, `">1.0"`, `"<2.0"`, `"<=1.9.0"`
/// - Wildcard: `"*"`
///
/// Rejects empty strings and strings that don't begin with a recognised
/// version-requirement prefix.
fn validate_version_requirement(dep_name: &str, spec: &str) -> Result<(), ManifestError> {
    const REASON: &str = "must be a non-empty version requirement such as \"1.0.0\", \
        \"^1.0\", \">=1.0.0\", \"~1.2\", or \"*\"";
    let field = format!("dependencies.{dep_name}");

    if spec.is_empty() {
        return Err(ManifestError::InvalidField {
            field,
            reason: REASON.into(),
        });
    }

    // Wildcard is always valid on its own.
    if spec == "*" {
        return Ok(());
    }

    // Strip recognised operator prefixes.  `!=` is intentionally excluded
    // as it is not part of the Pact version-requirement spec.
    let rest = if let Some(s) = spec.strip_prefix(">=").or_else(|| spec.strip_prefix("<=")) {
        s
    } else if let Some(s) = spec
        .strip_prefix('>')
        .or_else(|| spec.strip_prefix('<'))
        .or_else(|| spec.strip_prefix('^'))
        .or_else(|| spec.strip_prefix('~'))
    {
        s
    } else {
        spec
    };

    // What remains must start with a digit (begin of version number).
    if rest.is_empty() || !rest.starts_with(|c: char| c.is_ascii_digit()) {
        return Err(ManifestError::InvalidField {
            field,
            reason: REASON.into(),
        });
    }

    // Each dot-separated component (before any `-` or `+` suffix) must be
    // a non-empty sequence of ASCII digits.  A trailing `-` or `+` with an
    // empty suffix is also rejected here.
    let core = if let Some((c, meta)) = rest.split_once('+') {
        if meta.is_empty() {
            return Err(ManifestError::InvalidField {
                field,
                reason: "trailing `+` must be followed by non-empty build metadata".into(),
            });
        }
        c
    } else {
        rest
    };
    let core = if let Some((c, pre)) = core.split_once('-') {
        if pre.is_empty() {
            return Err(ManifestError::InvalidField {
                field,
                reason: "trailing `-` must be followed by non-empty pre-release identifier".into(),
            });
        }
        c
    } else {
        core
    };

    let version_parts_ok = core
        .split('.')
        .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()));

    if version_parts_ok {
        Ok(())
    } else {
        Err(ManifestError::InvalidField {
            field,
            reason: REASON.into(),
        })
    }
}

/// Validates a pact capability name in the `[pacts]` section.
///
/// A valid pact name:
/// - Is non-empty
/// - Starts with an ASCII uppercase letter (`PascalCase`)
/// - Contains only ASCII letters, digits, and underscores in the base name
/// - Optionally ends with a single parenthesised type parameter, e.g. `Breach(String)`
fn validate_pact_name(name: &str, field: &str) -> Result<(), ManifestError> {
    const REASON: &str = "pact names must start with an uppercase letter and contain only \
        ASCII letters, digits, and underscores (optionally followed by a type parameter \
        in parentheses, e.g. \"Breach(String)\")";

    if name.is_empty() {
        return Err(ManifestError::InvalidField {
            field: field.into(),
            reason: REASON.into(),
        });
    }

    // Check for optional trailing `(TypeParam)`.
    let (base, type_param) = if let Some(rest) = name.strip_suffix(')') {
        if let Some(paren_pos) = rest.rfind('(') {
            let base = &rest[..paren_pos];
            let param = &rest[paren_pos + 1..];
            (base, Some(param))
        } else {
            // Has `)` but no matching `(` — invalid.
            return Err(ManifestError::InvalidField {
                field: field.into(),
                reason: REASON.into(),
            });
        }
    } else {
        (name, None)
    };

    // Base name must be non-empty, start with uppercase, contain only letters/digits/underscores.
    if base.is_empty() {
        return Err(ManifestError::InvalidField {
            field: field.into(),
            reason: REASON.into(),
        });
    }

    let first = base.chars().next().expect("base is non-empty");
    if !first.is_ascii_uppercase() {
        return Err(ManifestError::InvalidField {
            field: field.into(),
            reason: REASON.into(),
        });
    }

    let base_ok = base.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if !base_ok {
        return Err(ManifestError::InvalidField {
            field: field.into(),
            reason: REASON.into(),
        });
    }

    // Type parameter (if present) must be non-empty and contain only
    // ASCII letters, digits, and underscores.
    if let Some(param) = type_param {
        if param.is_empty() {
            return Err(ManifestError::InvalidField {
                field: field.into(),
                reason: REASON.into(),
            });
        }
        let param_ok = param.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
        if !param_ok {
            return Err(ManifestError::InvalidField {
                field: field.into(),
                reason: REASON.into(),
            });
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::str::FromStr as _;

    use super::*;

    // -----------------------------------------------------------------------
    // Fixtures
    // -----------------------------------------------------------------------

    const FULL_MANIFEST: &str = r#"
[package]
name        = "weather-agent"
version     = "0.1.0"
authors     = ["you@example.com"]
license     = "MIT"
description = "A weather-aware scheduling agent"

[dependencies]
http-client = "1.2.0"
json        = "0.8.0"

[pacts]
required = ["Http", "Console", "ToolRegistry"]
optional = ["FileSystem"]

[seal]
max_memory    = "256mb"
max_duration  = "30s"
allowed_hosts = ["api.openweathermap.org"]
"#;

    const MINIMAL_MANIFEST: &str = r#"
[package]
name    = "my-app"
version = "1.0.0"
"#;

    // -----------------------------------------------------------------------
    // Happy-path tests
    // -----------------------------------------------------------------------

    #[test]
    fn parse_valid_complete_manifest() {
        let manifest = Manifest::from_str(FULL_MANIFEST).expect("should parse");
        insta::assert_debug_snapshot!(manifest);
    }

    #[test]
    fn parse_minimal_manifest() {
        let manifest = Manifest::from_str(MINIMAL_MANIFEST).expect("should parse minimal");

        assert_eq!(manifest.package.name, "my-app");
        assert_eq!(manifest.package.version, "1.0.0");
        assert!(manifest.package.authors.is_empty());
        assert!(manifest.package.license.is_none());
        assert!(manifest.package.description.is_none());
        assert!(manifest.dependencies.is_empty());
        assert!(manifest.pacts.required.is_empty());
        assert!(manifest.pacts.optional.is_empty());
        assert!(manifest.seal.is_none());
    }

    #[test]
    fn empty_dependencies_section() {
        let toml = "[package]\nname = \"a\"\nversion = \"0.1.0\"\n\n[dependencies]\n";
        let manifest = Manifest::from_str(toml).expect("empty deps should parse");
        assert!(manifest.dependencies.is_empty());
    }

    #[test]
    fn empty_pacts_section() {
        let toml = "[package]\nname = \"a\"\nversion = \"0.1.0\"\n\n[pacts]\n";
        let manifest = Manifest::from_str(toml).expect("empty pacts should parse");
        assert!(manifest.pacts.required.is_empty());
        assert!(manifest.pacts.optional.is_empty());
    }

    #[test]
    fn seal_section_is_optional() {
        let manifest = Manifest::from_str(MINIMAL_MANIFEST).expect("should parse");
        assert!(manifest.seal.is_none());
    }

    #[test]
    fn unknown_sections_are_ignored() {
        let toml = r#"
[package]
name    = "a"
version = "0.1.0"

[future_feature]
foo = "bar"
"#;
        // The TOML deserialiser must not reject unknown keys (deny_unknown_fields
        // is NOT set on RawManifest).
        let manifest = Manifest::from_str(toml)
            .expect("unknown sections should be silently ignored for forward compatibility");
        assert_eq!(manifest.package.name, "a");
        assert_eq!(manifest.package.version, "0.1.0");
        assert!(manifest.dependencies.is_empty());
        assert!(manifest.pacts.required.is_empty());
        assert!(manifest.seal.is_none());
    }

    #[test]
    fn version_with_prerelease_suffix() {
        let toml = "[package]\nname = \"a\"\nversion = \"1.0.0-alpha.1\"\n";
        let manifest = Manifest::from_str(toml).expect("pre-release version should parse");
        assert_eq!(manifest.package.version, "1.0.0-alpha.1");
    }

    #[test]
    fn version_with_build_metadata() {
        let toml = "[package]\nname = \"a\"\nversion = \"1.0.0+build.42\"\n";
        let manifest = Manifest::from_str(toml).expect("build-metadata version should parse");
        assert_eq!(manifest.package.version, "1.0.0+build.42");
    }

    /// `1.0.0-alpha.1+build.42` — pre-release combined with build metadata
    /// must be accepted.
    #[test]
    fn version_with_prerelease_and_build_metadata() {
        let toml = "[package]\nname = \"a\"\nversion = \"1.0.0-alpha.1+build.42\"\n";
        let manifest =
            Manifest::from_str(toml).expect("pre-release + build metadata version should parse");
        assert_eq!(manifest.package.version, "1.0.0-alpha.1+build.42");
    }

    // -----------------------------------------------------------------------
    // R2 P1: Trailing `-` / `+` with empty suffix must be rejected
    // -----------------------------------------------------------------------

    /// `1.0.0-` — trailing hyphen with no pre-release identifier is invalid.
    #[test]
    fn version_trailing_hyphen_is_error() {
        let toml = "[package]\nname = \"a\"\nversion = \"1.0.0-\"\n";
        let err = Manifest::from_str(toml).expect_err("\"1.0.0-\" should be rejected");
        assert!(
            matches!(err, ManifestError::InvalidField { ref field, .. } if field == "package.version"),
            "unexpected error: {err}"
        );
    }

    /// `1.0.0+` — trailing plus with no build metadata is invalid.
    #[test]
    fn version_trailing_plus_is_error() {
        let toml = "[package]\nname = \"a\"\nversion = \"1.0.0+\"\n";
        let err = Manifest::from_str(toml).expect_err("\"1.0.0+\" should be rejected");
        assert!(
            matches!(err, ManifestError::InvalidField { ref field, .. } if field == "package.version"),
            "unexpected error: {err}"
        );
    }

    // -----------------------------------------------------------------------
    // Error-case tests
    // -----------------------------------------------------------------------

    #[test]
    fn missing_package_section_is_error() {
        let toml = "[dependencies]\nfoo = \"1.0.0\"\n";
        let err = Manifest::from_str(toml).expect_err("should fail without [package]");
        assert!(
            matches!(err, ManifestError::MissingField(ref f) if f == "package"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn missing_name_field_is_error() {
        let toml = "[package]\nversion = \"0.1.0\"\n";
        let err = Manifest::from_str(toml).expect_err("should fail without name");
        assert!(
            matches!(err, ManifestError::MissingField(ref f) if f == "package.name"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn missing_version_field_is_error() {
        let toml = "[package]\nname = \"my-app\"\n";
        let err = Manifest::from_str(toml).expect_err("should fail without version");
        assert!(
            matches!(err, ManifestError::MissingField(ref f) if f == "package.version"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn empty_name_is_error() {
        let toml = "[package]\nname = \"\"\nversion = \"0.1.0\"\n";
        let err = Manifest::from_str(toml).expect_err("should fail with empty name");
        assert!(
            matches!(err, ManifestError::MissingField(ref f) if f == "package.name"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn name_with_spaces_is_error() {
        let toml = "[package]\nname = \"my app\"\nversion = \"0.1.0\"\n";
        let err = Manifest::from_str(toml).expect_err("name with spaces should fail");
        assert!(
            matches!(err, ManifestError::InvalidField { ref field, .. } if field == "package.name"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn name_with_uppercase_is_error() {
        let toml = "[package]\nname = \"MyApp\"\nversion = \"0.1.0\"\n";
        let err = Manifest::from_str(toml).expect_err("name with uppercase should fail");
        assert!(
            matches!(err, ManifestError::InvalidField { ref field, .. } if field == "package.name"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn invalid_version_format_is_error() {
        let cases = ["1.0", "v1.0.0", "1.0.0.0", "latest", ""];
        for bad in cases {
            let toml = format!("[package]\nname = \"a\"\nversion = \"{bad}\"\n");
            let err = Manifest::from_str(&toml)
                .expect_err(&format!("expected error for version={bad:?}"));
            assert!(
                matches!(err, ManifestError::InvalidField { ref field, .. } if field == "package.version"),
                "unexpected error for version={bad:?}: {err}"
            );
        }
    }

    #[test]
    fn from_path_nonexistent_file_is_io_error() {
        let err = Manifest::from_path("/nonexistent/path/pact.toml")
            .expect_err("nonexistent file should fail");
        assert!(
            matches!(err, ManifestError::IoError(ref path, _) if path == std::path::Path::new("/nonexistent/path/pact.toml")),
            "unexpected error: {err}"
        );
        // Verify the path appears in the formatted error message.
        let msg = err.to_string();
        assert!(
            msg.contains("/nonexistent/path/pact.toml"),
            "error message should contain the path, got: {msg}"
        );
    }

    // -----------------------------------------------------------------------
    // P0-1: Tightened package name validation
    // -----------------------------------------------------------------------

    #[test]
    fn name_leading_hyphen_is_error() {
        let toml = "[package]\nname = \"-foo\"\nversion = \"0.1.0\"\n";
        let err = Manifest::from_str(toml).expect_err("\"-foo\" should be rejected");
        assert!(
            matches!(err, ManifestError::InvalidField { ref field, .. } if field == "package.name"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn name_trailing_hyphen_is_error() {
        let toml = "[package]\nname = \"foo-\"\nversion = \"0.1.0\"\n";
        let err = Manifest::from_str(toml).expect_err("\"foo-\" should be rejected");
        assert!(
            matches!(err, ManifestError::InvalidField { ref field, .. } if field == "package.name"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn name_sole_hyphen_is_error() {
        let toml = "[package]\nname = \"-\"\nversion = \"0.1.0\"\n";
        let err = Manifest::from_str(toml).expect_err("\"-\" should be rejected");
        assert!(
            matches!(err, ManifestError::InvalidField { ref field, .. } if field == "package.name"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn name_consecutive_hyphens_is_error() {
        let toml = "[package]\nname = \"a--b\"\nversion = \"0.1.0\"\n";
        let err = Manifest::from_str(toml).expect_err("\"a--b\" should be rejected");
        assert!(
            matches!(err, ManifestError::InvalidField { ref field, .. } if field == "package.name"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn name_all_digits_is_error() {
        let toml = "[package]\nname = \"123\"\nversion = \"0.1.0\"\n";
        let err = Manifest::from_str(toml).expect_err("\"123\" (all digits) should be rejected");
        assert!(
            matches!(err, ManifestError::InvalidField { ref field, .. } if field == "package.name"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn name_digit_hyphen_letter_is_valid() {
        // e.g. "1a" or "a1b" — starts/ends with alnum and contains a letter
        let toml = "[package]\nname = \"a1b\"\nversion = \"0.1.0\"\n";
        Manifest::from_str(toml).expect("\"a1b\" should be a valid name");
    }

    // -----------------------------------------------------------------------
    // R2 P2: Additional edge-case tests — package name
    // -----------------------------------------------------------------------

    /// An underscore in a pact name is valid; `validate_pact_name` allows it.
    /// (The package name rules are separate; this test covers `[pacts]` names.)
    #[test]
    fn pact_name_with_underscore_is_valid() {
        let toml =
            "[package]\nname = \"a\"\nversion = \"0.1.0\"\n\n[pacts]\nrequired = [\"My_Tool\"]\n";
        Manifest::from_str(toml).expect("\"My_Tool\" should be a valid pact name");
    }

    /// Empty parentheses `Foo()` must be rejected: the type parameter is empty.
    #[test]
    fn pact_name_empty_parens_is_error() {
        let toml =
            "[package]\nname = \"a\"\nversion = \"0.1.0\"\n\n[pacts]\nrequired = [\"Foo()\"]\n";
        let err = Manifest::from_str(toml).expect_err("\"Foo()\" should be rejected");
        assert!(
            matches!(err, ManifestError::InvalidField { ref field, .. } if field == "pacts.required"),
            "unexpected error: {err}"
        );
    }

    // -----------------------------------------------------------------------
    // P0-2: Dependency version requirement validation
    // -----------------------------------------------------------------------

    #[test]
    fn dep_version_exact_semver_is_valid() {
        let toml =
            "[package]\nname = \"a\"\nversion = \"0.1.0\"\n\n[dependencies]\nfoo = \"1.2.3\"\n";
        Manifest::from_str(toml).expect("exact semver dep should parse");
    }

    #[test]
    fn dep_version_caret_is_valid() {
        let toml =
            "[package]\nname = \"a\"\nversion = \"0.1.0\"\n\n[dependencies]\nfoo = \"^1.0\"\n";
        Manifest::from_str(toml).expect("caret dep version should parse");
    }

    #[test]
    fn dep_version_tilde_is_valid() {
        let toml =
            "[package]\nname = \"a\"\nversion = \"0.1.0\"\n\n[dependencies]\nfoo = \"~1.2\"\n";
        Manifest::from_str(toml).expect("tilde dep version should parse");
    }

    #[test]
    fn dep_version_gte_is_valid() {
        let toml =
            "[package]\nname = \"a\"\nversion = \"0.1.0\"\n\n[dependencies]\nfoo = \">=1.0.0\"\n";
        Manifest::from_str(toml).expect(">= dep version should parse");
    }

    #[test]
    fn dep_version_wildcard_is_valid() {
        let toml = "[package]\nname = \"a\"\nversion = \"0.1.0\"\n\n[dependencies]\nfoo = \"*\"\n";
        Manifest::from_str(toml).expect("wildcard dep version should parse");
    }

    #[test]
    fn dep_version_empty_is_error() {
        let toml = "[package]\nname = \"a\"\nversion = \"0.1.0\"\n\n[dependencies]\nfoo = \"\"\n";
        let err = Manifest::from_str(toml).expect_err("empty dep version should fail");
        assert!(
            matches!(err, ManifestError::InvalidField { ref field, .. } if field == "dependencies.foo"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn dep_version_obviously_invalid_is_error() {
        let toml =
            "[package]\nname = \"a\"\nversion = \"0.1.0\"\n\n[dependencies]\nfoo = \"latest\"\n";
        let err = Manifest::from_str(toml).expect_err("\"latest\" dep version should fail");
        assert!(
            matches!(err, ManifestError::InvalidField { ref field, .. } if field == "dependencies.foo"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn dep_version_operator_only_is_error() {
        let toml = "[package]\nname = \"a\"\nversion = \"0.1.0\"\n\n[dependencies]\nfoo = \"^\"\n";
        let err = Manifest::from_str(toml).expect_err("bare \"^\" should fail");
        assert!(
            matches!(err, ManifestError::InvalidField { ref field, .. } if field == "dependencies.foo"),
            "unexpected error: {err}"
        );
    }

    // -----------------------------------------------------------------------
    // R2 P1: Trailing `-` / `+` in dependency version requirements
    // -----------------------------------------------------------------------

    /// `^1.0.0-` — trailing hyphen with no pre-release identifier is invalid.
    #[test]
    fn dep_version_trailing_hyphen_is_error() {
        let toml =
            "[package]\nname = \"a\"\nversion = \"0.1.0\"\n\n[dependencies]\nfoo = \"^1.0.0-\"\n";
        let err = Manifest::from_str(toml).expect_err("\"^1.0.0-\" should be rejected");
        assert!(
            matches!(err, ManifestError::InvalidField { ref field, .. } if field == "dependencies.foo"),
            "unexpected error: {err}"
        );
    }

    /// `1.0.0-+` — hyphen with empty pre-release before the `+` is invalid.
    #[test]
    fn dep_version_hyphen_then_plus_is_error() {
        let toml =
            "[package]\nname = \"a\"\nversion = \"0.1.0\"\n\n[dependencies]\nfoo = \"1.0.0-+\"\n";
        let err = Manifest::from_str(toml).expect_err("\"1.0.0-+\" should be rejected");
        assert!(
            matches!(err, ManifestError::InvalidField { ref field, .. } if field == "dependencies.foo"),
            "unexpected error: {err}"
        );
    }

    // -----------------------------------------------------------------------
    // R2 P2: `!=` operator must be rejected
    // -----------------------------------------------------------------------

    /// `!=1.0.0` is not a recognised Pact version-requirement operator.
    #[test]
    fn dep_version_not_equal_is_error() {
        let toml =
            "[package]\nname = \"a\"\nversion = \"0.1.0\"\n\n[dependencies]\nfoo = \"!=1.0.0\"\n";
        let err = Manifest::from_str(toml).expect_err("\"!=1.0.0\" should be rejected");
        assert!(
            matches!(err, ManifestError::InvalidField { ref field, .. } if field == "dependencies.foo"),
            "unexpected error: {err}"
        );
    }

    // -----------------------------------------------------------------------
    // R2 P2: Dependency pre-release version requirement
    // -----------------------------------------------------------------------

    /// `^1.0.0-rc.1` — caret requirement with a pre-release suffix is valid.
    #[test]
    fn dep_version_with_prerelease_is_valid() {
        let toml = "[package]\nname = \"a\"\nversion = \"0.1.0\"\n\n[dependencies]\nfoo = \"^1.0.0-rc.1\"\n";
        Manifest::from_str(toml).expect("\"^1.0.0-rc.1\" should be a valid version requirement");
    }

    // -----------------------------------------------------------------------
    // P1-5: Malformed TOML test
    // -----------------------------------------------------------------------

    #[test]
    fn malformed_toml_is_parse_error() {
        let err = Manifest::from_str("not valid { toml").expect_err("malformed TOML should fail");
        assert!(
            matches!(err, ManifestError::ParseError(_)),
            "unexpected error: {err}"
        );
    }

    // -----------------------------------------------------------------------
    // P2-13: Pact name validation in [pacts] section
    // -----------------------------------------------------------------------

    #[test]
    fn pact_name_pascal_case_is_valid() {
        let toml = "[package]\nname = \"a\"\nversion = \"0.1.0\"\n\n[pacts]\nrequired = [\"Http\", \"FileSystem\"]\n";
        Manifest::from_str(toml).expect("PascalCase pact names should parse");
    }

    #[test]
    fn pact_name_with_type_param_is_valid() {
        let toml = "[package]\nname = \"a\"\nversion = \"0.1.0\"\n\n[pacts]\nrequired = [\"Breach(String)\"]\n";
        Manifest::from_str(toml).expect("pact name with type param should parse");
    }

    #[test]
    fn pact_name_lowercase_is_error() {
        let toml =
            "[package]\nname = \"a\"\nversion = \"0.1.0\"\n\n[pacts]\nrequired = [\"http\"]\n";
        let err = Manifest::from_str(toml).expect_err("lowercase pact name should fail");
        assert!(
            matches!(err, ManifestError::InvalidField { ref field, .. } if field == "pacts.required"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn pact_name_empty_is_error() {
        let toml = "[package]\nname = \"a\"\nversion = \"0.1.0\"\n\n[pacts]\nrequired = [\"\"]\n";
        let err = Manifest::from_str(toml).expect_err("empty pact name should fail");
        assert!(
            matches!(err, ManifestError::InvalidField { ref field, .. } if field == "pacts.required"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn pact_name_digit_start_is_error() {
        let toml =
            "[package]\nname = \"a\"\nversion = \"0.1.0\"\n\n[pacts]\nrequired = [\"1Http\"]\n";
        let err = Manifest::from_str(toml).expect_err("digit-starting pact name should fail");
        assert!(
            matches!(err, ManifestError::InvalidField { ref field, .. } if field == "pacts.required"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn pact_name_with_hyphen_is_error() {
        let toml =
            "[package]\nname = \"a\"\nversion = \"0.1.0\"\n\n[pacts]\nrequired = [\"My-Pact\"]\n";
        let err = Manifest::from_str(toml).expect_err("hyphenated pact name should fail");
        assert!(
            matches!(err, ManifestError::InvalidField { ref field, .. } if field == "pacts.required"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn pact_name_optional_lowercase_is_error() {
        let toml = "[package]\nname = \"a\"\nversion = \"0.1.0\"\n\n[pacts]\noptional = [\"fileSystem\"]\n";
        let err = Manifest::from_str(toml).expect_err("lowercase optional pact name should fail");
        assert!(
            matches!(err, ManifestError::InvalidField { ref field, .. } if field == "pacts.optional"),
            "unexpected error: {err}"
        );
    }
}
