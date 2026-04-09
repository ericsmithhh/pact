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

use std::{collections::BTreeMap, path::Path};

use serde::Deserialize;
use thiserror::Error;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A fully-parsed and validated `pact.toml` manifest.
#[derive(Debug, Clone, PartialEq)]
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
#[derive(Debug, Clone, PartialEq)]
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
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PactRequirements {
    /// Capabilities that must be provided at runtime.
    pub required: Vec<String>,
    /// Capabilities that enhance functionality when available.
    pub optional: Vec<String>,
}

/// Resource-usage constraints imposed on the running package.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SealPolicy {
    /// Maximum memory allocation, e.g. `"256mb"`.
    pub max_memory: Option<String>,
    /// Maximum wall-clock duration, e.g. `"30s"`.
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

    /// A required field was absent or invalid.
    ///
    /// The inner string names the field (e.g. `"package.name"`).
    #[error("missing or invalid field `{0}`")]
    MissingField(String),

    /// The file could not be read from disk.
    #[error("could not read manifest file: {0}")]
    IoError(#[from] std::io::Error),
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
    /// [`ManifestError::MissingField`] if a required field is absent or
    /// violates naming/versioning rules.
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
    /// Returns [`ManifestError::IoError`] if the file cannot be read, and all
    /// errors from [`std::str::FromStr`] for parsing/validation failures.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, ManifestError> {
        let text = std::fs::read_to_string(path)?;
        text.parse()
    }
}

// ---------------------------------------------------------------------------
// Raw (deserialisation) layer — not part of the public API
// ---------------------------------------------------------------------------

/// Mirrors the TOML shape exactly; validation happens in `validate()`.
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

        let pacts = PactRequirements {
            required: self.pacts.required,
            optional: self.pacts.optional,
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

/// A valid package name is non-empty and contains only lowercase ASCII
/// letters, ASCII digits, and hyphens.
fn validate_package_name(name: &str) -> Result<(), ManifestError> {
    if name.is_empty() {
        return Err(ManifestError::MissingField("package.name".into()));
    }
    let valid = name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if valid {
        Ok(())
    } else {
        Err(ManifestError::MissingField("package.name".into()))
    }
}

/// A valid version string matches the pattern `DIGITS.DIGITS.DIGITS` with
/// optional pre-release / build-metadata suffixes.  We do a lightweight
/// structural check rather than full semver parsing.
fn validate_version(version: &str) -> Result<(), ManifestError> {
    // Accept `X.Y.Z` or `X.Y.Z-pre` or `X.Y.Z+meta` or both.
    // Strategy: strip optional `-…` and `+…` suffixes, then check `X.Y.Z`.
    let core = version.split_once('+').map_or(version, |(core, _)| core);
    let core = core.split_once('-').map_or(core, |(core, _)| core);

    let parts: Vec<&str> = core.split('.').collect();
    let ok = parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()));
    if ok {
        Ok(())
    } else {
        Err(ManifestError::MissingField("package.version".into()))
    }
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
        let result = Manifest::from_str(toml);
        assert!(
            result.is_ok(),
            "unknown sections should be silently ignored for forward compatibility"
        );
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
            matches!(err, ManifestError::MissingField(ref f) if f == "package.name"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn name_with_uppercase_is_error() {
        let toml = "[package]\nname = \"MyApp\"\nversion = \"0.1.0\"\n";
        let err = Manifest::from_str(toml).expect_err("name with uppercase should fail");
        assert!(
            matches!(err, ManifestError::MissingField(ref f) if f == "package.name"),
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
                matches!(err, ManifestError::MissingField(ref f) if f == "package.version"),
                "unexpected error for version={bad:?}: {err}"
            );
        }
    }

    #[test]
    fn from_path_nonexistent_file_is_io_error() {
        let err = Manifest::from_path("/nonexistent/path/pact.toml")
            .expect_err("nonexistent file should fail");
        assert!(
            matches!(err, ManifestError::IoError(_)),
            "unexpected error: {err}"
        );
    }
}
