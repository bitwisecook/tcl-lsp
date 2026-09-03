// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! `tclpkg.tcl` manifest loader.
//!
//! The manifest is a tiny Tcl
//! file evaluated under a whitelist of 13 directives; any other command is
//! refused with `command not permitted in safe mode: <cmd>`, as in a
//! safe Tcl interpreter. Because manifests are
//! pure data (no variable/command substitution), the script is parsed
//! into commands and words using Tcl grouping rules (braces, quotes,
//! backslash, comments, semicolons) and dispatches each directive directly —
//! no VM required.

use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;
use tcl_lexer::{Lexer, LexerConfig, Token, TokenType};

use crate::errors::TclPkgError;
use crate::version::Version;

// SPDX-style licence identifier, lightly validated.
static SPDX_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z0-9.\-+ ]+$").expect("spdx regex compiles"));

// Tcl runtime constraint syntax: optional operator + semver-ish version.
static TCL_CONSTRAINT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(>=|>|==|=|<=|<)?\s*([0-9][0-9a-zA-Z.+\-]*)\s*$")
        .expect("constraint regex compiles")
});

/// The manifest directive names, for cross-crate drift tests (the registry's
/// `TCLPKG_MANIFEST_ENV` scoped environment must describe exactly this set).
#[must_use]
pub fn directive_names() -> &'static [&'static str] {
    DIRECTIVES
}

const DIRECTIVES: &[&str] = &[
    "package",
    "version",
    "description",
    "license",
    "author",
    "homepage",
    "tcl",
    "require",
    "dev-require",
    "replace",
    "exclude",
    "provides",
    "entry",
    "build",
];

/// A single `require` / `dev-require` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requirement {
    pub name: String,
    pub minimum: Version,
    pub source_url: Option<String>,
    pub dev: bool,
}

/// A single `replace` directive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Replacement {
    pub name: String,
    pub version: Version,
    pub source_url: String,
}

/// A single `exclude` directive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exclusion {
    pub name: String,
    pub version: Version,
}

/// Parsed, validated representation of a `tclpkg.tcl` manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestAst {
    pub name: String,
    pub version: Option<Version>,
    pub description: String,
    pub license: String,
    pub authors: Vec<String>,
    pub homepage: String,
    pub tcl_constraint: String,
    pub requires: Vec<Requirement>,
    pub dev_requires: Vec<Requirement>,
    pub replaces: Vec<Replacement>,
    pub excludes: Vec<Exclusion>,
    pub provides: Vec<String>,
    pub entry: String,
    /// Optional build script (a Tcl file run, deprivileged, by `tcl pkg build`).
    /// Data only: declaring it never causes execution. It runs solely when an
    /// operator both enables build scripts and trusts the package.
    pub build: BuildDecl,
    pub path: String,
}

/// A declarative build-script declaration. The script path plus the (minimal)
/// capabilities it asks for; the sandbox grants only the intersection with
/// policy.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BuildDecl {
    /// Path to the build script, relative to the manifest. Empty when absent.
    pub script: String,
    /// Whether the build script requests network access.
    pub network: bool,
}

impl Default for ManifestAst {
    fn default() -> Self {
        Self {
            name: String::new(),
            version: None,
            description: String::new(),
            license: String::new(),
            authors: Vec::new(),
            homepage: String::new(),
            tcl_constraint: ">=8.6".to_string(),
            requires: Vec::new(),
            dev_requires: Vec::new(),
            replaces: Vec::new(),
            excludes: Vec::new(),
            provides: Vec::new(),
            entry: String::new(),
            build: BuildDecl::default(),
            path: String::new(),
        }
    }
}

/// Evaluate a manifest source string and return the AST.
pub fn load_manifest_text(source: &str, path: Option<&str>) -> Result<ManifestAst, TclPkgError> {
    let mut ast = ManifestAst {
        path: path.unwrap_or("").to_string(),
        ..ManifestAst::default()
    };

    let commands = parse_script(source).map_err(|m| manifest_err(&m, path))?;
    for command in commands {
        if command.is_empty() {
            continue;
        }
        let name = &command[0];
        if !DIRECTIVES.contains(&name.as_str()) {
            return Err(manifest_err(
                &format!("command not permitted in safe mode: {name}"),
                path,
            ));
        }
        let args = &command[1..];
        dispatch(&mut ast, name, args).map_err(|m| manifest_err(&m, path))?;
    }

    validate(&ast, path)?;
    Ok(ast)
}

/// Read a manifest file from disk and evaluate it.
pub fn load_manifest(path: impl AsRef<Path>) -> Result<ManifestAst, TclPkgError> {
    let file_path = path.as_ref();
    let text = match std::fs::read_to_string(file_path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(TclPkgError::manifest(
                format!("manifest not found: {}", file_path.display()),
                None,
                None,
                Some("run 'tcl pkg init' to create one".to_string()),
            ));
        }
        Err(e) => {
            return Err(TclPkgError::manifest(
                format!("cannot read manifest {}: {e}", file_path.display()),
                None,
                None,
                None,
            ));
        }
    };
    load_manifest_text(&text, Some(&file_path.to_string_lossy()))
}

fn manifest_err(message: &str, path: Option<&str>) -> TclPkgError {
    TclPkgError::manifest(message, path, None, None)
}

#[allow(clippy::too_many_lines)] // one arm per directive
fn dispatch(ast: &mut ManifestAst, name: &str, args: &[String]) -> Result<(), String> {
    match name {
        "package" => {
            require_arity("package", args, 1, 1)?;
            if !ast.name.is_empty() {
                return Err("package directive already set".to_string());
            }
            ast.name.clone_from(&args[0]);
        }
        "version" => {
            require_arity("version", args, 1, 1)?;
            ast.version = Some(parse_ver("version", &args[0])?);
        }
        "description" => {
            require_arity("description", args, 1, 1)?;
            ast.description.clone_from(&args[0]);
        }
        "license" => {
            require_arity("license", args, 1, 1)?;
            let value = &args[0];
            if !SPDX_RE.is_match(value) {
                return Err(format!("license: invalid SPDX-style identifier: '{value}'"));
            }
            ast.license.clone_from(value);
        }
        "author" => {
            require_arity("author", args, 1, 1)?;
            ast.authors.push(args[0].clone());
        }
        "homepage" => {
            require_arity("homepage", args, 1, 1)?;
            ast.homepage.clone_from(&args[0]);
        }
        "tcl" => {
            require_arity("tcl", args, 1, 1)?;
            let raw = &args[0];
            let Some(caps) = TCL_CONSTRAINT_RE.captures(raw) else {
                return Err(format!("tcl: invalid version constraint: '{raw}'"));
            };
            let _ = caps;
            let trimmed = raw.trim_start();
            let has_op = [">=", ">", "==", "=", "<=", "<"]
                .iter()
                .any(|op| trimmed.starts_with(op));
            ast.tcl_constraint = if has_op {
                raw.clone()
            } else {
                format!(">={}", raw.trim())
            };
        }
        "require" => {
            require_arity("require", args, 2, 4)?;
            let (remainder, source) = parse_source_option(args);
            if remainder.len() != 2 {
                return Err(
                    "wrong # args: \"require\" expects name, minimum, ?-source URL?".to_string(),
                );
            }
            ast.requires.push(Requirement {
                name: remainder[0].clone(),
                minimum: parse_ver("require", &remainder[1])?,
                source_url: source,
                dev: false,
            });
        }
        "dev-require" => {
            require_arity("dev-require", args, 2, 4)?;
            let (remainder, source) = parse_source_option(args);
            if remainder.len() != 2 {
                return Err(
                    "wrong # args: \"dev-require\" expects name, minimum, ?-source URL?"
                        .to_string(),
                );
            }
            ast.dev_requires.push(Requirement {
                name: remainder[0].clone(),
                minimum: parse_ver("dev-require", &remainder[1])?,
                source_url: source,
                dev: true,
            });
        }
        "replace" => {
            require_arity("replace", args, 3, 3)?;
            ast.replaces.push(Replacement {
                name: args[0].clone(),
                version: parse_ver("replace", &args[1])?,
                source_url: args[2].clone(),
            });
        }
        "exclude" => {
            require_arity("exclude", args, 2, 2)?;
            ast.excludes.push(Exclusion {
                name: args[0].clone(),
                version: parse_ver("exclude", &args[1])?,
            });
        }
        "provides" => {
            require_arity("provides", args, 1, 1)?;
            ast.provides.push(args[0].clone());
        }
        "entry" => {
            require_arity("entry", args, 1, 1)?;
            ast.entry.clone_from(&args[0]);
        }
        "build" => {
            // `build <script> ?-network?` — declaration only; never executed here.
            require_arity("build", args, 1, 2)?;
            if !ast.build.script.is_empty() {
                return Err("build directive already set".to_string());
            }
            ast.build.script.clone_from(&args[0]);
            for opt in &args[1..] {
                match opt.as_str() {
                    "-network" => ast.build.network = true,
                    other => return Err(format!("build: unknown option '{other}'")),
                }
            }
        }
        _ => unreachable!("dispatch only called for whitelisted directives"),
    }
    Ok(())
}

fn require_arity(
    name: &str,
    args: &[String],
    at_least: usize,
    at_most: usize,
) -> Result<(), String> {
    let count = args.len();
    if count < at_least || count > at_most {
        let expected = if at_least == at_most {
            format!("{at_least} arg(s)")
        } else {
            format!("between {at_least} and {at_most} args")
        };
        return Err(format!(
            "wrong # args: \"{name}\" expects {expected}, got {count}"
        ));
    }
    Ok(())
}

fn parse_ver(name: &str, raw: &str) -> Result<Version, String> {
    Version::parse(raw).map_err(|e| format!("{name}: {e}"))
}

fn parse_source_option(args: &[String]) -> (Vec<String>, Option<String>) {
    if args.len() >= 2 && args[args.len() - 2] == "-source" {
        let remainder = args[..args.len() - 2].to_vec();
        let source = args[args.len() - 1].clone();
        (remainder, Some(source))
    } else {
        (args.to_vec(), None)
    }
}

fn validate(ast: &ManifestAst, path: Option<&str>) -> Result<(), TclPkgError> {
    if ast.name.is_empty() {
        return Err(manifest_err(
            "manifest missing required 'package' directive",
            path,
        ));
    }
    if ast.version.is_none() {
        return Err(manifest_err(
            "manifest missing required 'version' directive",
            path,
        ));
    }
    let mut seen: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
    for req in &ast.requires {
        seen.insert(req.name.as_str(), "require");
    }
    for req in &ast.dev_requires {
        if seen.contains_key(req.name.as_str()) {
            return Err(manifest_err(
                &format!(
                    "package '{}' declared both as require and dev-require",
                    req.name
                ),
                path,
            ));
        }
        seen.insert(req.name.as_str(), "dev-require");
    }
    Ok(())
}

// Tcl script parsing (commands + words; no substitution)

/// Split a manifest script into commands, each a list of literal words.
///
/// The lexer owns Tcl's structural grouping rules (including nested command
/// substitutions in quotes). This loader deliberately only decodes grouping /
/// backslashes afterwards: it never evaluates a variable or command
/// substitution.
fn parse_script(src: &str) -> Result<Vec<Vec<String>>, String> {
    let tokens = Lexer::with_config(
        src,
        LexerConfig {
            strict_quoting: true,
            // dialect-drift-ok: a `tclpkg.tcl` manifest DSL, not a Tcl
            // document — a fixed declarative vocabulary with no dialect axis.
            ..LexerConfig::default()
        },
    )
    .tokenise_all()
    .map_err(|err| err.to_string())?;
    let mut commands = Vec::new();
    let mut words = Vec::new();
    let mut word = Vec::new();

    for token in tokens {
        match token.kind {
            TokenType::Comment => {}
            TokenType::Expand => {
                return Err(
                    "manifest expansion prefix `{*}` is not permitted in safe mode".to_string(),
                );
            }
            TokenType::Sep => {
                finish_word(src, &mut word, token.span.start() as usize, &mut words)?;
            }
            TokenType::Eol | TokenType::Eof => {
                finish_word(src, &mut word, token.span.start() as usize, &mut words)?;
                if !words.is_empty() {
                    commands.push(std::mem::take(&mut words));
                }
            }
            _ => word.push(token),
        }
    }
    Ok(commands)
}

/// Finish one lexer-grouped word at a separator / command boundary.
fn finish_word(
    src: &str,
    word: &mut Vec<Token>,
    end: usize,
    words: &mut Vec<String>,
) -> Result<(), String> {
    if word.is_empty() {
        return Ok(());
    }
    let start = word[0].span.start() as usize;
    let raw = src
        .get(start..end)
        .ok_or_else(|| "invalid lexer token span".to_string())?;
    let value = if word.len() == 1 && word[0].kind == TokenType::Str {
        raw.strip_prefix('{')
            .and_then(|s| s.strip_suffix('}'))
            .map_or_else(
                || Err("missing close-brace".to_string()),
                |s| Ok(s.to_owned()),
            )?
    } else if raw.starts_with('"') {
        raw.strip_prefix('"')
            .and_then(|s| s.strip_suffix('"'))
            .map_or_else(
                || Err("missing \"".to_string()),
                |s| Ok(tcl_syntax::backslash::decode(s).into_owned()),
            )?
    } else {
        tcl_syntax::backslash::decode(raw).into_owned()
    };
    words.push(value);
    word.clear();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(src: &str) -> Result<ManifestAst, TclPkgError> {
        load_manifest_text(src, None)
    }

    #[test]
    fn minimal_manifest() {
        let ast = load("package myapp\nversion 1.0.0\n").unwrap();
        assert_eq!(ast.name, "myapp");
        assert_eq!(ast.version, Some(Version::parse("1.0.0").unwrap()));
        assert_eq!(ast.tcl_constraint, ">=8.6");
        assert!(ast.requires.is_empty());
    }

    #[test]
    fn missing_required_fields() {
        assert!(
            load("version 1.0.0\n")
                .unwrap_err()
                .to_string()
                .contains("required 'package'")
        );
        assert!(
            load("package myapp\n")
                .unwrap_err()
                .to_string()
                .contains("required 'version'")
        );
        assert!(load("").is_err());
    }

    #[test]
    fn duplicate_package() {
        let err = load("package a\npackage b\nversion 1.0.0\n").unwrap_err();
        assert!(err.to_string().contains("already set"));
    }

    #[test]
    fn require_directive() {
        let ast = load("package myapp\nversion 1.0.0\nrequire json 1.3.5\n").unwrap();
        assert_eq!(ast.requires.len(), 1);
        assert_eq!(ast.requires[0].name, "json");
        assert_eq!(ast.requires[0].minimum, Version::parse("1.3.5").unwrap());
        assert!(ast.requires[0].source_url.is_none());

        let ast = load(
            "package myapp\nversion 1.0.0\nrequire json 1.3.5 -source https://example.org/json.tar.gz\n",
        )
        .unwrap();
        assert_eq!(
            ast.requires[0].source_url.as_deref(),
            Some("https://example.org/json.tar.gz")
        );
    }

    #[test]
    fn dev_require_and_conflict() {
        let ast = load("package myapp\nversion 1.0.0\ndev-require tcltest 2.5\n").unwrap();
        assert!(ast.requires.is_empty());
        assert_eq!(ast.dev_requires.len(), 1);
        assert!(ast.dev_requires[0].dev);

        let err = load("package myapp\nversion 1.0.0\nrequire json 1.0\ndev-require json 1.0\n")
            .unwrap_err();
        assert!(err.to_string().contains("both as require and dev-require"));
    }

    #[test]
    fn require_invalid_version() {
        let err = load("package myapp\nversion 1.0.0\nrequire json garbage\n").unwrap_err();
        assert!(err.to_string().contains("invalid version"));
    }

    #[test]
    fn replace_exclude() {
        let ast =
            load("package a\nversion 1.0.0\nreplace json 1.4.0 https://example.org/json.tar.gz\n")
                .unwrap();
        assert_eq!(ast.replaces[0].name, "json");
        assert_eq!(
            ast.replaces[0].source_url,
            "https://example.org/json.tar.gz"
        );

        let ast = load("package a\nversion 1.0.0\nexclude http 2.9.7\n").unwrap();
        assert_eq!(ast.excludes[0].name, "http");
        assert_eq!(ast.excludes[0].version, Version::parse("2.9.7").unwrap());
    }

    #[test]
    fn tcl_constraint() {
        assert_eq!(
            load("package a\nversion 1.0.0\ntcl >=9.0\n")
                .unwrap()
                .tcl_constraint,
            ">=9.0"
        );
        assert_eq!(
            load("package a\nversion 1.0.0\ntcl 8.6\n")
                .unwrap()
                .tcl_constraint,
            ">=8.6"
        );
        assert!(
            load("package a\nversion 1.0.0\ntcl @@@\n")
                .unwrap_err()
                .to_string()
                .contains("invalid version constraint")
        );
    }

    #[test]
    fn metadata_braces_and_quotes() {
        let ast = load(
            "package a\nversion 1.0.0\ndescription {Example}\nlicense MIT\nhomepage https://example.org\n",
        )
        .unwrap();
        assert_eq!(ast.description, "Example");
        assert_eq!(ast.license, "MIT");
        assert_eq!(ast.homepage, "https://example.org");

        let ast =
            load("package a\nversion 1.0.0\nauthor {Alice <a@x>}\nauthor {Bob <b@x>}\n").unwrap();
        assert_eq!(ast.authors, vec!["Alice <a@x>", "Bob <b@x>"]);

        let ast = load("package a\nversion 1.0.0\ndescription \"A test package\"\n").unwrap();
        assert_eq!(ast.description, "A test package");

        let ast = load(
            "package a\nversion 1.0.0\nprovides a::serve\nprovides a::client\nentry main.tcl\n",
        )
        .unwrap();
        assert_eq!(ast.provides, vec!["a::serve", "a::client"]);
        assert_eq!(ast.entry, "main.tcl");
    }

    #[test]
    fn license_rejects_garbage() {
        let err = load("package a\nversion 1.0.0\nlicense <bad>\n").unwrap_err();
        assert!(err.to_string().contains("invalid SPDX"));
    }

    #[test]
    fn safe_mode_refuses_forbidden() {
        for forbidden in [
            "exec ls /",
            "open /etc/passwd",
            "source /tmp/x.tcl",
            "puts stderr hi",
            "file delete /tmp/x",
            "socket localhost 80",
        ] {
            let body = format!("package a\nversion 1.0.0\n{forbidden}\n");
            let err = load(&body).unwrap_err();
            assert!(
                err.to_string().contains("not permitted in safe mode"),
                "expected safe-mode rejection for {forbidden:?}, got {err}"
            );
        }
    }

    #[test]
    fn comments_and_semicolons() {
        let ast = load("# a comment\npackage a ; version 1.0.0\n# trailing\n").unwrap();
        assert_eq!(ast.name, "a");
        assert_eq!(ast.version, Some(Version::parse("1.0.0").unwrap()));
    }

    #[test]
    fn quoted_nested_command_substitution_is_one_literal_word() {
        let ast = load(
            r#"package a
version 1.0.0
description "See [string map {a b} \"note\"] for details"
"#,
        )
        .unwrap();
        assert_eq!(
            ast.description,
            r#"See [string map {a b} "note"] for details"#
        );
    }

    #[test]
    fn substitutions_remain_literal_in_safe_manifest_words() {
        let ast = load(
            r#"package a
version 1.0.0
description "$value [dangerous command]"
"#,
        )
        .unwrap();
        assert_eq!(ast.description, "$value [dangerous command]");
    }

    #[test]
    fn expansion_prefix_is_rejected_in_safe_manifest_words() {
        let err = load(
            r"package a
version 1.0.0
description {*}$value
",
        )
        .unwrap_err();
        assert!(
            err.to_string()
                .contains("expansion prefix `{*}` is not permitted")
        );
    }
}
