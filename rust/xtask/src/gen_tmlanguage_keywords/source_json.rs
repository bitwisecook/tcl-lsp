// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Exact source replacement through serde's borrowed JSON value owner.

use std::collections::BTreeMap;
use std::fmt;

use anyhow::{Context, Result, ensure};
use serde::de::{Error, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::value::RawValue;

pub(super) enum PathPart<'a> {
    Key(&'a str),
    Index(usize),
}

struct SourceObject<'a>(BTreeMap<String, &'a RawValue>);

impl<'de> Deserialize<'de> for SourceObject<'de> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ObjectVisitor;
        impl<'de> Visitor<'de> for ObjectVisitor {
            type Value = SourceObject<'de>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an object with unique decoded keys")
            }

            fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
                let mut fields = BTreeMap::new();
                while let Some(key) = map.next_key::<String>()? {
                    let value = map.next_value::<&'de RawValue>()?;
                    if fields.insert(key, value).is_some() {
                        return Err(M::Error::custom("duplicate source JSON key"));
                    }
                }
                Ok(SourceObject(fields))
            }
        }
        deserializer.deserialize_map(ObjectVisitor)
    }
}

pub(super) fn replace_string(
    text: &str,
    path: &[PathPart<'_>],
    replacement: &str,
) -> Result<String> {
    let mut value: &RawValue = serde_json::from_str(text)?;
    for component in path {
        value = match component {
            PathPart::Key(key) => {
                let object: SourceObject<'_> = serde_json::from_str(value.get())?;
                *object.0.get(*key).context("missing source JSON field")?
            }
            PathPart::Index(index) => {
                let array: Vec<&RawValue> = serde_json::from_str(value.get())?;
                *array.get(*index).context("missing source JSON element")?
            }
        };
    }
    let _: String = serde_json::from_str(value.get())?;
    // Borrowed RawValue slices are issued by serde from this input. Authenticate
    // their actual source extent as well, without searching equal rendered text.
    let start = value
        .get()
        .as_ptr()
        .addr()
        .checked_sub(text.as_ptr().addr())
        .context("JSON source slice is outside the original input")?;
    let end = start
        .checked_add(value.get().len())
        .context("JSON source extent overflow")?;
    ensure!(
        text.get(start..end) == Some(value.get()),
        "foreign JSON source extent"
    );
    let encoded = serde_json::to_string(replacement)?;
    Ok(format!("{}{}{}", &text[..start], encoded, &text[end..]))
}

#[cfg(test)]
mod tests {
    use super::{
        PathPart::{Index, Key},
        replace_string,
    };

    #[test]
    fn exact_path_does_not_replace_equal_decoys_or_reordered_fields() {
        let source = r#"{"repository":{"target":{"decoy":{"match":"same"},"patterns":[{"captures":{"0":{"match":"same"}},"match":"same"}]},"elsewhere":{"match":"same"}}}"#;
        let changed = replace_string(
            source,
            &[
                Key("repository"),
                Key("target"),
                Key("patterns"),
                Index(0),
                Key("match"),
            ],
            "changed",
        )
        .unwrap();
        let json: serde_json::Value = serde_json::from_str(&changed).unwrap();
        assert_eq!(
            json["repository"]["target"]["patterns"][0]["match"],
            "changed"
        );
        assert_eq!(json["repository"]["target"]["decoy"]["match"], "same");
        assert_eq!(
            json["repository"]["target"]["patterns"][0]["captures"]["0"]["match"],
            "same"
        );
        assert_eq!(json["repository"]["elsewhere"]["match"], "same");
        assert_eq!(changed.matches("changed").count(), 1);
    }

    #[test]
    fn decoded_keys_and_noncanonical_string_escapes_keep_original_source_extent() {
        let source = r#"{ "ma\u0074ch" : "\u0073ame", "decoy": "same" }"#;
        assert_eq!(
            replace_string(source, &[Key("match")], "next").unwrap(),
            r#"{ "ma\u0074ch" : "next", "decoy": "same" }"#
        );
        assert!(
            replace_string(r#"{"match":"a","ma\u0074ch":"b"}"#, &[Key("match")], "next").is_err()
        );
        assert!(replace_string(r#"{"other":"same"}"#, &[Key("match")], "next").is_err());
        assert!(replace_string(r#"{"match":{}}"#, &[Key("match")], "next").is_err());
    }
}
