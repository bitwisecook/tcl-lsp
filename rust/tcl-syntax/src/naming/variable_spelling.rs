// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Scalar source spelling with independent source and Native purposes.

use super::{NativeNameProtocol, NativeNameQualification};

/// A literal name word and a complete variable reference that reproduce the
/// same native operand. This grants no cell, name availability, variable
/// lifetime, observer, command selection or refactoring permission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeScalarSourceSpelling {
    name_word: String,
    reference: String,
}

impl NativeScalarSourceSpelling {
    #[must_use]
    pub fn name_word(&self) -> &str {
        &self.name_word
    }

    #[must_use]
    pub fn reference(&self) -> &str {
        &self.reference
    }
}

/// Propose one complete scalar reference under the selected source grammar.
/// Names remain literal data; this supplies no native name recipe, cell, read,
/// current value, source eligibility or edit permission.
#[must_use]
pub fn scalar_source_reference(
    name: &str,
    config: tcl_lexer::LexerConfig,
    require_braces: bool,
) -> Option<String> {
    if !require_braces && super::is_bare_var_name(name) {
        let reference = format!("${name}");
        if super::whole_scalar_variable_reference_name(&reference, config) == Some(name) {
            return Some(reference);
        }
    }
    let reference = format!("${{{name}}}");
    (super::whole_scalar_variable_reference_name(&reference, config) == Some(name))
        .then_some(reference)
}

/// Propose one scalar component without absorbing any part of the complete
/// following source fragment. The same configured scanner must reproduce the
/// exact name and boundary after a braced retry. This is source syntax only.
#[must_use]
pub fn scalar_source_reference_followed_by(
    name: &str,
    following: &str,
    config: tcl_lexer::LexerConfig,
) -> Option<String> {
    for require_braces in [false, true] {
        let Some(reference) = scalar_source_reference(name, config, require_braces) else {
            continue;
        };
        let joined = format!("{reference}{following}");
        if tcl_lexer::word_parts::scan_var_ref(joined.as_bytes(), 0, config).is_ok_and(|parsed| {
            parsed.is_some_and(|parsed| {
                parsed.next == reference.len()
                    && parsed.name == name.as_bytes()
                    && parsed.index.is_none()
            })
        }) {
            return Some(reference);
        }
    }
    None
}

/// Propose a complete array reference from separate literal root/index data.
/// The selected scanner and static fragment decoder must reproduce both parts.
/// This supplies syntax only, without source ownership, a cell or a read grant.
#[must_use]
pub fn array_source_reference(
    root: &str,
    index: &str,
    config: tcl_lexer::LexerConfig,
    require_braces: bool,
) -> Option<String> {
    if !require_braces {
        let reference = format!("${root}({index})");
        if let Ok(Some(parsed)) = tcl_lexer::word_parts::whole_var_ref(reference.as_bytes(), config)
            && parsed.name == root.as_bytes()
            && let Some(source) = parsed
                .index
                .and_then(|index| std::str::from_utf8(index).ok())
            && crate::word_rules::standalone_static_word_fragment_value(source, false, config)
                .as_deref()
                == Some(index)
        {
            return Some(reference);
        }
    }
    let name = format!("{root}({index})");
    let reference = format!("${{{name}}}");
    let parsed = tcl_lexer::word_parts::whole_var_ref(reference.as_bytes(), config).ok()??;
    if parsed.index.is_some() {
        return None;
    }
    let (selected_root, selected_index) = super::split_element_ref_bytes(parsed.name)?;
    (selected_root == root.as_bytes() && selected_index == index.as_bytes()).then_some(reference)
}

/// Replace one whole original reference's written tail while preserving its
/// qualifier, wrappers and literal or computed index bytes. The selected
/// scanner must reproduce the complete proposed name, root and index shape;
/// an unrepresentable proposal declines. This supplies no source currency,
/// selected variable, cell, lookup, read or edit permission.
#[must_use]
pub fn variable_source_reference_with_tail(
    source: &str,
    tail: &str,
    config: tcl_lexer::LexerConfig,
) -> Option<String> {
    let bytes = source.as_bytes();
    let original = tcl_lexer::word_parts::whole_var_ref(bytes, config).ok()??;
    let name_range = original.name_range_in(bytes)?;
    let root = super::variable_reference_root_bytes(bytes, config).ok()??;
    let qualifier_len = root
        .len()
        .checked_sub(super::written_command_tail(root).len())?;
    let qualifier = std::str::from_utf8(root.get(..qualifier_len)?).ok()?;
    let proposed_root = format!("{qualifier}{tail}");
    if super::written_command_tail(proposed_root.as_bytes()) != tail.as_bytes() {
        return None;
    }
    let suffix = std::str::from_utf8(original.name.get(root.len()..)?).ok()?;
    let proposed_name = format!("{proposed_root}{suffix}");
    let start = name_range.start.checked_add(qualifier_len)?;
    let end = name_range.start.checked_add(root.len())?;
    let proposal = format!("{}{tail}{}", source.get(..start)?, source.get(end..)?);
    let parsed = tcl_lexer::word_parts::whole_var_ref(proposal.as_bytes(), config).ok()??;
    let matches = parsed.name == proposed_name.as_bytes()
        && parsed.index == original.index
        && super::variable_reference_root_bytes(proposal.as_bytes(), config).ok()??
            == proposed_root.as_bytes();
    matches.then_some(proposal)
}

/// Render an exact local scalar name and reference under the complete source
/// grammar and native string recipe. Qualification, array interpretation,
/// clipped operands and unrepresentable source text are unavailable. The
/// shared variable scanner validates the complete reference independently.
#[must_use]
pub fn native_scalar_source_spelling(
    original: &[u8],
    channel: tcl_lexer::SourceChannel,
    config: tcl_lexer::LexerConfig,
    protocol: NativeNameProtocol,
) -> Option<NativeScalarSourceSpelling> {
    let input = protocol.combined_variable_input(original);
    if input.element().is_some()
        || input.root().qualification() != NativeNameQualification::Unqualified
        || input.root().selected() != original
    {
        return None;
    }
    let literal = crate::backslash::native_literal_source_text(
        original,
        channel,
        protocol.string_protocol(),
    )?;
    let reference = format!("${{{literal}}}");
    let parsed = tcl_lexer::word_parts::scan_var_ref(reference.as_bytes(), 0, config).ok()??;
    if parsed.next != reference.len() || parsed.index.is_some() {
        return None;
    }
    let reproduced = crate::backslash::native_source_literal_bytes(
        parsed.name,
        channel,
        protocol.string_protocol(),
    )
    .ok()?;
    if reproduced.as_ref() != original {
        return None;
    }
    let name_word = crate::backslash::native_literal_source_word(
        original,
        channel,
        config,
        protocol.string_protocol(),
    )?;
    Some(NativeScalarSourceSpelling {
        name_word,
        reference,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_dialect::DialectProfile;
    use tcl_lexer::{LexerConfig, SourceChannel};

    #[test]
    fn scalar_source_components_check_the_complete_following_fragment() {
        for dialect in ["tcl8.4", "tcl8.6", "tcl9.0", "jim"] {
            let config = LexerConfig::for_dialect(dialect);
            for (following, expected) in [
                ("", "$x"),
                ("tail", "${x}"),
                ("::tail", "${x}"),
                ("(key)", "${x}"),
                ("[puts next]", "$x"),
                ("$next", "$x"),
                ("\\ntext", "$x"),
            ] {
                assert_eq!(
                    scalar_source_reference_followed_by("x", following, config).as_deref(),
                    Some(expected),
                    "{dialect} {following}"
                );
            }
            assert!(scalar_source_reference_followed_by("x}tail", "suffix", config).is_none());
        }
    }

    #[test]
    fn scalar_source_proposals_preserve_native_operand_and_complete_reference() {
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let point = tcl_dialect::model::DialectPoint::of_dialect_name(Some(dialect)).unwrap();
            let profile = if dialect == "jim" {
                DialectProfile::projected_from_point("jim", &[], "Jim", point)
            } else {
                DialectProfile::find(dialect).unwrap().clone()
            };
            let protocol = NativeNameProtocol::for_point(point).unwrap();
            let config = LexerConfig::for_profile(Some(&profile));
            let value = crate::backslash::native_source_literal_bytes(
                "café".as_bytes(),
                SourceChannel::Document,
                protocol.string_protocol(),
            )
            .unwrap();
            let proposal =
                native_scalar_source_spelling(&value, SourceChannel::Document, config, protocol)
                    .unwrap();
            assert_eq!(proposal.reference(), "${café}");
            for unavailable in [b"::x".as_slice(), b"a(x)", b"x}suffix", b"\xed\xa0\x80"] {
                assert!(
                    native_scalar_source_spelling(
                        unavailable,
                        SourceChannel::Document,
                        config,
                        protocol,
                    )
                    .is_none(),
                    "{dialect} {unavailable:?}"
                );
            }
            assert_eq!(
                native_scalar_source_spelling(
                    b"x\0tail",
                    SourceChannel::Document,
                    config,
                    protocol,
                )
                .is_some(),
                protocol.is_jim084(),
                "{dialect}",
            );
        }
    }
    #[test]
    fn source_array_references_keep_literal_indices_and_selected_delimiters() {
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let config = LexerConfig::for_dialect(environment);
            assert_eq!(
                array_source_reference("arr", "key", config, false).as_deref(),
                Some("$arr(key)")
            );
            for index in ["$literal", "[list redirected]", r"\x41", ")", ""] {
                let reference = array_source_reference("arr", index, config, true).unwrap();
                assert_eq!(
                    reference,
                    format!("${{arr({index})}}"),
                    "{environment}: {index}"
                );
            }
            for index in ["$literal", "[list redirected]", r"\x41"] {
                let reference = array_source_reference("arr", index, config, false).unwrap();
                assert_eq!(
                    reference,
                    format!("${{arr({index})}}"),
                    "{environment}: {index}"
                );
            }
            let nested = config.braced_var == tcl_dialect::BracedVarStyle::Tcl9Nesting;
            assert_eq!(
                array_source_reference("arr", "a{b}c", config, true).is_some(),
                nested,
                "{environment}"
            );
            assert_eq!(
                array_source_reference("arr", "a{b", config, true).is_some(),
                !nested,
                "{environment}"
            );
            assert!(array_source_reference("bad root", "key", config, false).is_some());
            assert!(array_source_reference("arr", "unmatched}suffix", config, true).is_none());
        }
    }

    #[test]
    fn source_scalar_references_keep_selected_braces_and_literal_name_data() {
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let config = LexerConfig::for_dialect(environment);
            assert_eq!(
                scalar_source_reference("alpha", config, false).as_deref(),
                Some("$alpha")
            );
            assert_eq!(
                scalar_source_reference("alpha", config, true).as_deref(),
                Some("${alpha}")
            );
            for name in [
                "",
                "two words",
                "$literal",
                "[literal]",
                r"a\b",
                "a\nb",
                "a(b",
                "::rooted",
            ] {
                let reference = scalar_source_reference(name, config, true).unwrap();
                assert_eq!(reference, format!("${{{name}}}"), "{environment}");
            }
            let nested = config.braced_var == tcl_dialect::BracedVarStyle::Tcl9Nesting;
            assert_eq!(
                scalar_source_reference("a{b}c", config, false).is_some(),
                nested,
                "{environment}"
            );
            assert_eq!(
                scalar_source_reference("a{b", config, true).is_some(),
                !nested,
                "{environment}"
            );
            for name in ["a_}_closebrace", "array(key)"] {
                assert!(
                    scalar_source_reference(name, config, false).is_none(),
                    "{environment} {name}"
                );
            }
        }
    }
}

#[cfg(test)]
mod original_tail_proposal_tests {
    use super::*;

    #[test]
    fn original_reference_tail_proposals_keep_wrappers_indices_and_oracle_brace_grammar() {
        for style in [
            tcl_lexer::BracedVarStyle::FirstClose,
            tcl_lexer::BracedVarStyle::Tcl9Nesting,
        ] {
            let config = tcl_lexer::LexerConfig {
                braced_var: style,
                ..Default::default()
            };
            for (source, expected) in [
                ("$x", "$new"),
                ("${x}", "${new}"),
                ("$ns:::x($i[command]\\key)", "$ns:::new($i[command]\\key)"),
                ("${ns::x($literal[key])}", "${ns::new($literal[key])}"),
                ("${scalar(open}", "${new}"),
                ("${cash$name}", "${new}"),
                ("${}", "${new}"),
            ] {
                assert_eq!(
                    variable_source_reference_with_tail(source, "new", config).as_deref(),
                    Some(expected)
                );
            }
            for source in ["${missing", "$a tail", "${a}(tail)", "literal"] {
                assert!(variable_source_reference_with_tail(source, "new", config).is_none());
            }
            assert!(
                variable_source_reference_with_tail("${ns::x}", "other::new", config).is_none()
            );
            assert!(variable_source_reference_with_tail("${ns::x}", ":new", config).is_none());
            assert!(variable_source_reference_with_tail("$x", "new(index)", config).is_none());
        }
        // Fixed original C5 and pinned Jim observations precede these controls:
        // FirstClose reads ns{::x, while Tcl9Nesting reads ns{::x}.
        let first = tcl_lexer::LexerConfig {
            braced_var: tcl_lexer::BracedVarStyle::FirstClose,
            ..Default::default()
        };
        let nested = tcl_lexer::LexerConfig {
            braced_var: tcl_lexer::BracedVarStyle::Tcl9Nesting,
            ..first
        };
        assert_eq!(
            variable_source_reference_with_tail("${ns{::x}", "new", first).as_deref(),
            Some("${ns{::new}")
        );
        assert!(variable_source_reference_with_tail("${ns{::x}}", "new", nested).is_none());
        assert_eq!(
            variable_source_reference_with_tail("${ns{::x}}", "new}", nested).as_deref(),
            Some("${ns{::new}}")
        );
    }
}
