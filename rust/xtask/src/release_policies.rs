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

//! Extract policy observations and project them into typed release facts.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail, ensure};
use tcl_dialect::TclVersion;

const DATA: &str = "rust/tcl-dialect/data/release-policies.tsv";
const OUTPUT: &str = "rust/tcl-dialect/src/generated_release_policy.rs";
const PROBE: &str = include_str!("../../tcl-dialect/data/release-policies.tcl");
const OBSERVATIONS: &[&str] = &[
    "patchlevel",
    "binary-number",
    "decimal-prefix",
    "escape-hex",
    "escape-wide",
    "escape-octal-length",
    "characters",
    "float-default",
    "float-precision",
    "expr-comments",
    "namespace-fallback",
    "regex-z-anchor",
    "uplevel-plus",
    "uplevel-negative",
    "upvar-parity",
    "compiled-borrowed-frame-slots",
    "compiled-counted-scalar-tokens",
    "compiled-array-base-reservation",
];

type Observations = BTreeMap<String, String>;

fn c_function<'a>(source: &'a str, name: &str) -> Option<&'a str> {
    let signature = format!("\n{name}(");
    let after = source.split_once(&signature)?.1;
    let body = after.split_once("\n{")?.1;
    body.split_once("\n}\n").map(|(body, _)| body)
}

fn compiled_policies(tree: &Path, rows: &mut Observations) -> Result<()> {
    let compiler = fs::read_to_string(tree.join("generic/tclCompile.c"))?;
    let commands = fs::read_to_string(tree.join("generic/tclCompCmds.c"))?;
    let locals =
        c_function(&compiler, "TclFindCompiledLocal").context("compiled local function")?;
    ensure!(
        locals.contains("procPtr") && locals.contains("strncmp("),
        "unrecognised compiled-local comparison"
    );
    let borrowed = locals.contains("localCachePtr");
    let scalar = [&compiler, &commands].into_iter().find_map(|source| {
        c_function(source, "TclLocalScalar").or_else(|| c_function(source, "LocalScalar"))
    });
    let counted = if let Some(body) = scalar {
        ensure!(
            body.contains("token[1].size = numBytes") && body.contains("ScalarFromToken("),
            "unrecognised counted scalar compiler"
        );
        true
    } else {
        ensure!(
            commands.contains("TclIsLocalScalar("),
            "unrecognised legacy scalar compiler"
        );
        false
    };
    let scalar_token = [&compiler, &commands].into_iter().find_map(|source| {
        c_function(source, "TclLocalScalarFromToken")
            .or_else(|| c_function(source, "LocalScalarFromToken"))
    });
    let reserves = if let Some(body) = scalar_token {
        let prepare = body.find("PushVarName(").context("scalar preparation")?;
        let decline = body.find("if (!isScalar)").context("scalar decline")?;
        ensure!(prepare < decline, "unrecognised scalar preparation order");
        true
    } else {
        false
    };
    rows.insert(
        "compiled-borrowed-frame-slots".into(),
        u8::from(borrowed).to_string(),
    );
    rows.insert(
        "compiled-counted-scalar-tokens".into(),
        u8::from(counted).to_string(),
    );
    rows.insert(
        "compiled-array-base-reservation".into(),
        u8::from(reserves).to_string(),
    );
    Ok(())
}

fn observe(root: &Path, version: TclVersion) -> Result<Observations> {
    let tree = root.join("tmp").join(version.source_tree_name());
    let mut child = Command::new(tree.join("unix/tclsh"))
        .env("TCL_LIBRARY", tree.join("library"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("run policy reference {}", tree.display()))?;
    child
        .stdin
        .take()
        .context("policy probe stdin")?
        .write_all(PROBE.as_bytes())?;
    let output = child.wait_with_output()?;
    ensure!(
        output.status.success() && output.stderr.is_empty(),
        "policy probe failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = std::str::from_utf8(&output.stdout)?;
    let mut rows = Observations::new();
    for line in text.lines() {
        let (key, value) =
            tcl_test_support::fixture::tsv_split_once(line).context("policy probe row")?;
        ensure!(
            rows.insert(key.to_owned(), value.to_owned()).is_none(),
            "duplicate policy observation"
        );
    }
    compiled_policies(&tree, &mut rows)?;
    validate(&rows, version)?;
    Ok(rows)
}

fn validate(rows: &Observations, version: TclVersion) -> Result<()> {
    ensure!(
        rows.len() == OBSERVATIONS.len() && OBSERVATIONS.iter().all(|key| rows.contains_key(*key)),
        "incomplete or unknown policy observations"
    );
    ensure!(
        rows.get("patchlevel").map(String::as_str) == Some(version.patchlevel()),
        "wrong policy reference build"
    );
    Ok(())
}

fn extract(root: &Path) -> Result<String> {
    let mut text = String::from("# Tcl release\tpolicy observation\tmeasured value\n");
    for version in TclVersion::ALL {
        for (key, value) in observe(root, version)? {
            writeln!(text, "{}\t{key}\t{value}", version.version_string())?;
        }
    }
    Ok(text)
}

fn parse(data: &str) -> Result<BTreeMap<TclVersion, Observations>> {
    let mut releases: BTreeMap<TclVersion, Observations> = BTreeMap::new();
    for line in data
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
    {
        let fields: Vec<_> = tcl_test_support::fixture::tsv_fields(line).collect();
        ensure!(fields.len() == 3, "policy facts require three fields");
        let version =
            TclVersion::from_version_string(fields[0]).context("unknown policy release")?;
        ensure!(
            releases
                .entry(version)
                .or_default()
                .insert(fields[1].to_owned(), fields[2].to_owned())
                .is_none(),
            "duplicate policy fact"
        );
    }
    ensure!(
        releases.len() == TclVersion::ALL.len(),
        "missing policy release"
    );
    for (&version, rows) in &releases {
        validate(rows, version)?;
    }
    Ok(releases)
}

fn value<'a>(rows: &'a Observations, key: &str) -> Result<&'a str> {
    rows.get(key)
        .map(String::as_str)
        .with_context(|| format!("missing {key}"))
}

fn boolean(rows: &Observations, key: &str) -> Result<bool> {
    match value(rows, key)? {
        "0" => Ok(false),
        "1" => Ok(true),
        other => bail!("{key}: expected boolean observation, got {other}"),
    }
}

fn render_policy(rows: &Observations, text: &mut String) -> Result<()> {
    let numbers = match (
        boolean(rows, "binary-number")?,
        boolean(rows, "decimal-prefix")?,
    ) {
        (false, false) => "Tcl84",
        (true, false) => "Tcl85",
        (true, true) => "Tcl90",
        (false, true) => bail!("unmodelled numeral grammar"),
    };
    let escapes = match (
        value(rows, "escape-hex")?,
        value(rows, "escape-wide")?,
        value(rows, "escape-octal-length")?,
    ) {
        ("B", "85", "1") => "Tcl84",
        ("A42", "65533", "2") => "Tcl86",
        ("A42", "128512", "2") => "Tcl90",
        _ => bail!("unmodelled escape grammar"),
    };
    let characters = match value(rows, "characters")? {
        "4" => "BmpCharsElseUtf8Bytes",
        "2" => "Utf16CodeUnits",
        "1" => "UnicodeScalars",
        _ => bail!("unmodelled character unit"),
    };
    let doubles = match (
        value(rows, "float-default")?,
        value(rows, "float-precision")?,
    ) {
        ("0.333333333333", "0.333333") => "Tcl84Precision",
        ("0.3333333333333333", "0.333333") => "TclPrecision",
        ("0.3333333333333333", "0.3333333333333333") => "Shortest",
        _ => bail!("unmodelled float precision policy"),
    };
    let comments = if boolean(rows, "expr-comments")? {
        "Hash"
    } else {
        "None"
    };
    let upvar = if boolean(rows, "upvar-parity")? {
        "ArgumentParity"
    } else {
        "DigitOrHash"
    };
    let uplevel = match (
        boolean(rows, "uplevel-plus")?,
        boolean(rows, "uplevel-negative")?,
    ) {
        (false, false) => "DigitOrHash",
        (true, false) => "DigitOrNonNegativeInteger",
        (true, true) => "IntegerOrHash",
        (false, true) => bail!("unmodelled level grammar"),
    };
    writeln!(
        text,
        "ReleasePolicyFacts {{ numbers: crate::NumberSyntax::{numbers}, escapes: crate::EscapeSyntax::{escapes}, characters: crate::StringCharacterModel::{characters}, double_string: crate::DoubleStringPolicy::{doubles}, expr_comments: crate::ExprCommentStyle::{comments}, upvar_level_presence: crate::FrameLevelPresence::{upvar}, uplevel_level_presence: crate::FrameLevelPresence::{uplevel},"
    )?;
    for (field, key) in [
        ("namespace_var_global_fallback", "namespace-fallback"),
        ("regex_z_anchor", "regex-z-anchor"),
    ] {
        writeln!(text, "{field}: {},", boolean(rows, key)?)?;
    }
    writeln!(text, "compiled_locals: CompiledLocalPolicy {{")?;
    for (field, key) in [
        ("borrow_frame_slots", "compiled-borrowed-frame-slots"),
        ("counted_scalar_tokens", "compiled-counted-scalar-tokens"),
        ("reserves_array_base", "compiled-array-base-reservation"),
    ] {
        writeln!(text, "{field}: {},", boolean(rows, key)?)?;
    }
    writeln!(text, "}}, }},")?;
    Ok(())
}

fn render(data: &str) -> Result<String> {
    let mut text = String::from(
        "// @generated by cargo xtask release-facts; do not edit.\nconst RELEASE_POLICIES: &[ReleasePolicyFacts] = &[\n",
    );
    for (_, rows) in parse(data)? {
        render_policy(&rows, &mut text)?;
    }
    text.push_str("];\n");
    crate::util::format_rust(&text)
}

pub fn run(check: bool, verify_sources: bool) -> Result<()> {
    let root = crate::util::repo_root();
    if verify_sources {
        let measured = extract(&root)?;
        if check {
            ensure!(
                fs::read_to_string(root.join(DATA))? == measured,
                "release policy observations differ from source/oracles"
            );
        } else {
            crate::util::write_if_changed(&root.join(DATA), measured)?;
        }
    }
    let rendered = render(&fs::read_to_string(root.join(DATA))?)?;
    if check {
        ensure!(
            fs::read_to_string(root.join(OUTPUT))? == rendered,
            "release policy Rust is stale"
        );
    } else {
        crate::util::write_if_changed(&root.join(OUTPUT), rendered)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn incomplete_duplicate_and_unknown_policy_data_are_rejected() {
        for data in [
            "",
            "8.4\tpatchlevel\t8.4.20\n8.4\tpatchlevel\t8.4.20\n",
            "8.7\tpatchlevel\t8.7.0\n",
            "8.4\tgarbage\t1\n",
        ] {
            assert!(parse(data).is_err());
        }
    }
}
