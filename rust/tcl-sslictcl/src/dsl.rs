// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Safe parser for the declarative `.sslictcl` vocabulary.
//!
//! Tcl supplies quoting, comments, line continuation, and nested braced
//! blocks.  This module walks the canonical CST and never invokes a Tcl
//! interpreter. Command substitution, variable substitution, and argument
//! expansion are rejected instead of being evaluated or guessed.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use tcl_compiler::parsing::syntax::build::build_document;
use tcl_compiler::parsing::syntax::segment::segments_from_document;
use tcl_lexer::{LexerConfig, SourceMap, TokenType, script_is_complete};

use crate::model::{
    CertificateDeclaration, Endpoint, HstsPolicy, ProtocolVersion, SslicModel, TlsValue,
};
use crate::testssl::import_testssl_json;

/// A successfully loaded document and its forwards-compatibility notices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DslDocument {
    /// Byte-for-byte original source. Unknown declarations can be re-emitted
    /// from this rather than from the normalized semantic projection.
    pub raw_source: String,
    /// Typed declarations.
    pub model: SslicModel,
    /// Unknown declarations retained in the model.
    pub notices: Vec<DslNotice>,
}

/// A non-fatal unknown declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DslNotice {
    /// One-based source line.
    pub line: u32,
    /// Human-readable explanation.
    pub message: String,
}

/// A syntax or schema error that makes a declaration ambiguous or unsafe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DslError {
    /// One-based source line.
    pub line: u32,
    /// Human-readable explanation.
    pub message: String,
}

impl fmt::Display for DslError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl Error for DslError {}

#[derive(Debug, Clone)]
struct Word {
    text: String,
    braced: bool,
    literal: bool,
}

#[derive(Debug, Clone)]
struct Stmt {
    words: Vec<Word>,
    line: u32,
}

impl Stmt {
    fn name(&self) -> &str {
        self.words.first().map_or("", |word| word.text.as_str())
    }

    fn word(&self, index: usize) -> Result<&Word, DslError> {
        self.words.get(index).ok_or_else(|| DslError {
            line: self.line,
            message: format!("`{}` is missing argument {index}", self.name()),
        })
    }

    fn require_words(&self, count: usize) -> Result<(), DslError> {
        if self.words.len() == count {
            Ok(())
        } else {
            Err(DslError {
                line: self.line,
                message: format!(
                    "`{}` expects {} word(s), got {}",
                    self.name(),
                    count,
                    self.words.len()
                ),
            })
        }
    }

    fn require_literals(&self) -> Result<(), DslError> {
        if self.words.iter().all(|word| word.literal) {
            Ok(())
        } else {
            Err(DslError {
                line: self.line,
                message: format!(
                    "`{}` must be declarative; substitutions and argument expansion are forbidden",
                    self.name()
                ),
            })
        }
    }
}

fn statements(source: &str, base_line: u32) -> Result<Vec<Stmt>, DslError> {
    if !script_is_complete(source) {
        return Err(DslError {
            line: base_line,
            message: "incomplete Tcl declaration or unclosed delimiter".to_owned(),
        });
    }
    let source_map = SourceMap::new(source);
    let (document, warnings) = build_document(source, LexerConfig::default());
    let mut line_starts = vec![0];
    for (offset, byte) in source.bytes().enumerate() {
        if byte == b'\n' {
            line_starts.push(offset + 1);
        }
    }
    let line_of = |offset: usize| {
        let index = match line_starts.binary_search(&offset) {
            Ok(index) => index,
            Err(index) => index.saturating_sub(1),
        };
        base_line + u32::try_from(index).unwrap_or(u32::MAX)
    };

    if let Some(warning) = warnings.first() {
        return Err(DslError {
            line: line_of(warning.offset as usize),
            message: format!("invalid Tcl declaration: {}", warning.message),
        });
    }

    Ok(segments_from_document(document, &source_map)
        .into_iter()
        .map(|segment| {
            let line = line_of(segment.span.start() as usize);
            let words = segment
                .texts
                .into_iter()
                .enumerate()
                .map(|(index, text)| {
                    let token = segment.argv[index];
                    let fragments = &segment.word_fragments[index];
                    let literal = !segment
                        .expand_word
                        .as_ref()
                        .and_then(|expanded| expanded.get(index))
                        .copied()
                        .unwrap_or(false)
                        && fragments.iter().all(|fragment| {
                            matches!(fragment.token.kind, TokenType::Esc | TokenType::Str)
                        });
                    Word {
                        text,
                        braced: fragments.len() == 1
                            && token.kind == TokenType::Str
                            && !token.in_quote,
                        literal,
                    }
                })
                .collect();
            Stmt { words, line }
        })
        .collect())
}

fn block(word: &Word, line: u32) -> Result<Vec<Stmt>, DslError> {
    if !word.braced {
        return Err(DslError {
            line,
            message: "declaration body must be a braced literal".to_owned(),
        });
    }
    statements(&word.text, line)
}

fn literal_list(word: &Word, line: u32) -> Result<Vec<String>, DslError> {
    if !word.literal {
        return Err(DslError {
            line,
            message: "list must not contain substitutions or argument expansion".to_owned(),
        });
    }
    if word.text.trim().is_empty() {
        return Ok(Vec::new());
    }
    if !word.braced {
        return Ok(vec![word.text.clone()]);
    }
    let rows = statements(&word.text, line)?;
    if rows.len() != 1 {
        return Err(DslError {
            line,
            message: "list must be a single Tcl list, not multiple commands".to_owned(),
        });
    }
    rows[0].require_literals()?;
    Ok(rows[0].words.iter().map(|item| item.text.clone()).collect())
}

fn extension_value(stmt: &Stmt) -> TlsValue {
    if stmt.words.len() == 2 {
        TlsValue::Scalar(stmt.words[1].text.clone())
    } else {
        TlsValue::List(
            stmt.words
                .iter()
                .skip(1)
                .map(|word| word.text.clone())
                .collect(),
        )
    }
}

fn add_extension(
    map: &mut BTreeMap<String, Vec<TlsValue>>,
    stmt: &Stmt,
    notices: &mut Vec<DslNotice>,
) {
    map.entry(stmt.name().to_owned())
        .or_default()
        .push(extension_value(stmt));
    notices.push(DslNotice {
        line: stmt.line,
        message: format!("unknown declaration `{}` preserved", stmt.name()),
    });
}

/// Load one `.sslictcl` source string without evaluating it.
pub fn load(source: &str) -> Result<DslDocument, DslError> {
    let mut model = SslicModel::default();
    let mut notices = Vec::new();
    let rows = statements(source, 1)?;
    let mut saw_header = false;

    for stmt in &rows {
        stmt.require_literals()?;
        match stmt.name() {
            "sslictcl" => {
                stmt.require_words(2)?;
                if saw_header {
                    return Err(DslError {
                        line: stmt.line,
                        message: "duplicate `sslictcl` vocabulary declaration".to_owned(),
                    });
                }
                model.vocabulary = stmt.word(1)?.text.parse().map_err(|_| DslError {
                    line: stmt.line,
                    message: "SslicTcl vocabulary must be an integer".to_owned(),
                })?;
                if model.vocabulary != 1 {
                    notices.push(DslNotice {
                        line: stmt.line,
                        message: format!(
                            "vocabulary {} is newer than supported vocabulary 1; unknown declarations will be preserved",
                            model.vocabulary
                        ),
                    });
                }
                saw_header = true;
            }
            "certificate" => parse_certificate(stmt, &mut model, &mut notices)?,
            "endpoint" => parse_endpoint(stmt, &mut model, &mut notices)?,
            "testssl-import" => parse_testssl_import(stmt, &mut model)?,
            "" => {}
            _ => add_extension(&mut model.extensions, stmt, &mut notices),
        }
    }

    if !saw_header {
        return Err(DslError {
            line: 1,
            message: "missing `sslictcl VERSION` declaration".to_owned(),
        });
    }
    Ok(DslDocument {
        raw_source: source.to_owned(),
        model,
        notices,
    })
}

fn parse_testssl_import(stmt: &Stmt, model: &mut SslicModel) -> Result<(), DslError> {
    stmt.require_words(3)?;
    let name = stmt.word(1)?.text.clone();
    let fields = block(stmt.word(2)?, stmt.line)?;
    let mut schema = None;
    let mut raw_json = None;
    for field in fields {
        field.require_literals()?;
        field.require_words(2)?;
        match field.name() {
            "schema" => schema = Some(field.word(1)?.text.clone()),
            "raw-json-hex" => {
                let bytes = decode_hex(&field.word(1)?.text).map_err(|message| DslError {
                    line: field.line,
                    message,
                })?;
                raw_json = Some(String::from_utf8(bytes).map_err(|_| DslError {
                    line: field.line,
                    message: "`raw-json-hex` is not UTF-8 JSON".to_owned(),
                })?);
            }
            other => {
                return Err(DslError {
                    line: field.line,
                    message: format!("unknown `testssl-import` field `{other}`"),
                });
            }
        }
    }
    if schema.as_deref() != Some("1") {
        return Err(DslError {
            line: stmt.line,
            message: "`testssl-import` schema must be 1".to_owned(),
        });
    }
    let source = raw_json.ok_or_else(|| DslError {
        line: stmt.line,
        message: "`testssl-import` requires `raw-json-hex`".to_owned(),
    })?;
    let imported = import_testssl_json(&source).map_err(|error| DslError {
        line: stmt.line,
        message: error.to_string(),
    })?;
    if model
        .testssl_imports
        .insert(name.clone(), imported)
        .is_some()
    {
        return Err(DslError {
            line: stmt.line,
            message: format!("duplicate testssl import `{name}`"),
        });
    }
    Ok(())
}

fn decode_hex(value: &str) -> Result<Vec<u8>, String> {
    if !value.len().is_multiple_of(2) {
        return Err("`raw-json-hex` must contain an even number of digits".to_owned());
    }
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let text = std::str::from_utf8(pair).unwrap_or_default();
            u8::from_str_radix(text, 16)
                .map_err(|_| "`raw-json-hex` contains a non-hex digit".to_owned())
        })
        .collect()
}

fn parse_certificate(
    stmt: &Stmt,
    model: &mut SslicModel,
    notices: &mut Vec<DslNotice>,
) -> Result<(), DslError> {
    stmt.require_words(3)?;
    let name = stmt.word(1)?.text.clone();
    if model.certificates.contains_key(&name) {
        return Err(DslError {
            line: stmt.line,
            message: format!("duplicate certificate `{name}`"),
        });
    }
    let mut declaration = CertificateDeclaration {
        name: name.clone(),
        material: String::new(),
        key: None,
        extensions: BTreeMap::new(),
    };
    for field in block(stmt.word(2)?, stmt.line)? {
        field.require_literals()?;
        match field.name() {
            "pem" | "material" => {
                field.require_words(2)?;
                declaration.material.clone_from(&field.word(1)?.text);
            }
            "key" => {
                field.require_words(2)?;
                declaration.key = Some(field.word(1)?.text.clone());
            }
            "" => {}
            _ => add_extension(&mut declaration.extensions, &field, notices),
        }
    }
    if declaration.material.is_empty() {
        return Err(DslError {
            line: stmt.line,
            message: format!("certificate `{name}` has no `pem` or `material` field"),
        });
    }
    model.certificates.insert(name, declaration);
    Ok(())
}

fn parse_endpoint(
    stmt: &Stmt,
    model: &mut SslicModel,
    notices: &mut Vec<DslNotice>,
) -> Result<(), DslError> {
    stmt.require_words(3)?;
    let name = stmt.word(1)?.text.clone();
    if model.endpoints.contains_key(&name) {
        return Err(DslError {
            line: stmt.line,
            message: format!("duplicate endpoint `{name}`"),
        });
    }
    let mut endpoint = Endpoint {
        name: name.clone(),
        ..Endpoint::default()
    };
    for field in block(stmt.word(2)?, stmt.line)? {
        field.require_literals()?;
        match field.name() {
            "hostname" => {
                field.require_words(2)?;
                endpoint.hostname = Some(field.word(1)?.text.clone());
            }
            "protocols" => {
                field.require_words(2)?;
                endpoint.protocols = literal_list(field.word(1)?, field.line)?
                    .iter()
                    .map(|value| {
                        value
                            .parse::<ProtocolVersion>()
                            .map_err(|message| DslError {
                                line: field.line,
                                message,
                            })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                endpoint.protocols.sort_unstable();
                endpoint.protocols.dedup();
            }
            "ciphers" => {
                field.require_words(2)?;
                endpoint.ciphers = literal_list(field.word(1)?, field.line)?;
            }
            "groups" => {
                field.require_words(2)?;
                endpoint.groups = literal_list(field.word(1)?, field.line)?;
            }
            "signature-schemes" => {
                field.require_words(2)?;
                endpoint.signature_schemes = literal_list(field.word(1)?, field.line)?;
            }
            "certificate-chain" => {
                field.require_words(2)?;
                endpoint.certificate_chain = literal_list(field.word(1)?, field.line)?;
            }
            "hsts" => endpoint.hsts = Some(parse_hsts(&field)?),
            "" => {}
            _ => add_extension(&mut endpoint.extensions, &field, notices),
        }
    }
    model.endpoints.insert(name, endpoint);
    Ok(())
}

fn parse_hsts(stmt: &Stmt) -> Result<HstsPolicy, DslError> {
    stmt.require_words(2)?;
    let mut policy = HstsPolicy::default();
    for field in block(stmt.word(1)?, stmt.line)? {
        field.require_literals()?;
        field.require_words(2)?;
        let value = &field.word(1)?.text;
        match field.name() {
            "enabled" => policy.enabled = parse_bool(value, field.line)?,
            "max-age" => {
                policy.max_age = Some(value.parse().map_err(|_| DslError {
                    line: field.line,
                    message: "HSTS `max-age` must be an integer".to_owned(),
                })?);
            }
            "include-subdomains" => policy.include_subdomains = parse_bool(value, field.line)?,
            "preload" => policy.preload = parse_bool(value, field.line)?,
            other => {
                return Err(DslError {
                    line: field.line,
                    message: format!("unknown HSTS field `{other}`"),
                });
            }
        }
    }
    Ok(policy)
}

fn parse_bool(value: &str, line: u32) -> Result<bool, DslError> {
    match value.to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" | "1" => Ok(true),
        "false" | "no" | "off" | "0" => Ok(false),
        _ => Err(DslError {
            line,
            message: format!("expected boolean, got `{value}`"),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r"
sslictcl 1
certificate leaf {
    pem {-----BEGIN CERTIFICATE-----
not-real
-----END CERTIFICATE-----}
    key leaf-key
    future-field {kept exactly}
}
endpoint /Common/example {
    hostname example.test
    protocols {tls1.2 tls1.3 tls1.2}
    ciphers {TLS_AES_128_GCM_SHA256 ECDHE-RSA-AES128-GCM-SHA256}
    certificate-chain {leaf intermediate}
    hsts {
        enabled true
        max-age 31536000
        include-subdomains yes
        preload false
    }
    adapter-source bigip
}
future-top value
";

    #[test]
    fn loads_literal_document_and_preserves_unknowns() {
        let loaded = load(SAMPLE).expect("valid DSL");
        let endpoint = &loaded.model.endpoints["/Common/example"];
        assert_eq!(
            endpoint.protocols,
            vec![ProtocolVersion::Tls12, ProtocolVersion::Tls13]
        );
        assert_eq!(endpoint.certificate_chain, ["leaf", "intermediate"]);
        assert_eq!(
            endpoint.hsts.as_ref().and_then(|hsts| hsts.max_age),
            Some(31_536_000)
        );
        assert_eq!(loaded.notices.len(), 3);
        assert!(loaded.model.extensions.contains_key("future-top"));
    }

    #[test]
    fn rejects_all_substitution_forms() {
        for source in [
            "sslictcl 1\nendpoint x { hostname $host }",
            "sslictcl 1\nendpoint x { hostname [exec id] }",
            "sslictcl 1\nendpoint x { ciphers {*}$values }",
        ] {
            let error = load(source).expect_err("dynamic DSL must be rejected");
            assert!(
                error.message.contains("declarative") || error.message.contains("substitution")
            );
        }
    }

    #[test]
    fn requires_header_and_unique_names() {
        assert!(load("endpoint x {}").is_err());
        assert!(load("sslictcl 1\nendpoint x {}\nendpoint x {}").is_err());
        assert!(load("sslictcl 1\nendpoint x {").is_err());
    }
}
