// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Versioned venv metadata preserves complete ordered decoded pairs.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::errors::TclPkgError;

const MARKER_PREFIX: &str = "# tclpkg venv config";
const MARKER: &str = "# tclpkg venv config TOML 1";
const RESERVED: &[&str] = &[
    "tcl_version",
    "tcl_executable",
    "venv_tool",
    "created",
    "prompt",
    "include-system-site-packages",
    "project_root",
];

fn validate_reserved(pairs: &[(String, String)]) -> Result<(), TclPkgError> {
    let mut seen = std::collections::HashSet::new();
    for (name, _) in pairs {
        if RESERVED.contains(&name.as_str()) && !seen.insert(name) {
            return Err(super::venv_error(
                format!("ambiguous duplicate venv config key: {name}"),
                None,
            ));
        }
    }
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    entries: Vec<Entry>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    name: String,
    value: String,
}

pub(super) fn path_value(path: &Path) -> Result<String, TclPkgError> {
    let value = path
        .to_str()
        .ok_or_else(|| super::venv_error("path has no Unicode venv-config representation", None))?;
    if value.as_bytes().contains(&0) {
        return Err(super::venv_error(
            "venv-config path cannot contain NUL",
            None,
        ));
    }
    Ok(value.to_owned())
}

pub(super) fn decode(source: &str) -> Result<Vec<(String, String)>, TclPkgError> {
    let first = source.lines().next().unwrap_or("");
    if first == MARKER {
        let document: Document = toml::from_str(source).map_err(|error| {
            super::venv_error(format!("invalid venv TOML config: {error}"), None)
        })?;
        let pairs = document
            .entries
            .into_iter()
            .map(|entry| (entry.name, entry.value))
            .collect::<Vec<_>>();
        validate_reserved(&pairs)?;
        return Ok(pairs);
    }
    if first.starts_with(MARKER_PREFIX) {
        return Err(super::venv_error("unsupported venv config format", None));
    }
    // Unmarked files retain the original line-format interpretation. A marked
    // parse failure never enters this compatibility path.
    Ok(source
        .lines()
        .filter_map(|line| {
            let (key, value) = line.split_once('=')?;
            Some((key.trim().to_owned(), value.trim().to_owned()))
        })
        .collect())
}

pub(super) fn encode(pairs: &[(String, String)]) -> Result<String, TclPkgError> {
    let document = Document {
        entries: pairs
            .iter()
            .map(|(name, value)| Entry {
                name: name.clone(),
                value: value.clone(),
            })
            .collect(),
    };
    let body = toml::to_string_pretty(&document).map_err(|error| {
        super::venv_error(format!("cannot encode venv TOML config: {error}"), None)
    })?;
    let source = format!("{MARKER}\n{body}");
    if decode(&source)?.as_slice() != pairs {
        return Err(super::venv_error(
            "venv config serialization changed original metadata",
            None,
        ));
    }
    Ok(source)
}

pub(super) fn replace_selected(
    pairs: &mut Vec<(String, String)>,
    version: &str,
    executable: &Path,
) -> Result<(), TclPkgError> {
    let executable = path_value(executable)?;
    validate_reserved(pairs)?;
    for (selected, value) in [
        ("tcl_version", version),
        ("tcl_executable", executable.as_str()),
    ] {
        let first = pairs.iter().position(|(key, _)| key == selected);
        if let Some(index) = first {
            value.clone_into(&mut pairs[index].1);
        } else {
            pairs.push((selected.to_owned(), value.to_owned()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn venv_config_roundtrips_ordered_original_pairs_without_key_or_value_interpolation() {
        let pairs = vec![
            (
                "prompt".into(),
                "literal \"quote\"\\path\nnext = data\r\n".into(),
            ),
            (
                "tcl_executable".into(),
                "/actual \"path\"\nnext=literal".into(),
            ),
            ("unrelated".into(), "first".into()),
            ("unrelated".into(), "second".into()),
            ("name\nwith=syntax".into(), "Unicode café".into()),
        ];
        let source = encode(&pairs).unwrap();
        assert_eq!(decode(&source).unwrap(), pairs);
        assert_eq!(
            decode("prompt = \"literal\"\nunknown = a=b\n").unwrap(),
            vec![
                ("prompt".into(), "\"literal\"".into()),
                ("unknown".into(), "a=b".into())
            ]
        );
        for body in [
            "broken",
            "entries = 1",
            "[[entries]]\nname=\"x\"\nvalue=4",
            "entries=[]\nunknown=\"data\"",
        ] {
            assert!(decode(&format!("{MARKER}\n{body}\n")).is_err());
        }
        assert!(decode("# tclpkg venv config TOML 2\nkey = value\n").is_err());
    }

    #[test]
    fn venv_config_updates_only_exact_selected_keys_and_refuses_ambiguous_selection() {
        let mut pairs = decode(
            "tcl_version_extra = keep\nprompt = \"literal\"\nunknown = first\nunknown = second\n",
        )
        .unwrap();
        replace_selected(&mut pairs, "9.0\nliteral=data", Path::new("/actual\npath")).unwrap();
        assert_eq!(pairs[0], ("tcl_version_extra".into(), "keep".into()));
        assert_eq!(decode(&encode(&pairs).unwrap()).unwrap(), pairs);
        for key in ["tcl_version", "tcl_executable"] {
            let mut ambiguous = vec![(key.into(), "one".into()), (key.into(), "two".into())];
            assert!(replace_selected(&mut ambiguous, "9.0", Path::new("/actual")).is_err());
        }
    }
}
