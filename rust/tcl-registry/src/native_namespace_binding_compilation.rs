// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original C global/variable compiler binding and preparation order.
//!
//! Compile-known local tails are declaration geometry. They establish no live
//! variable cell, handler, callback, native name-object cache or body entry.

use crate::native_compilation::{
    NativeCompilationContext, NativeCompilationFrame, NativeCompilationMode,
    NativeCompilationWordShape,
};
use crate::native_compiler_word_projection::{
    NativeCompilerProjectionUnavailable, NativeCompilerWordOperand, NativeProjectedCompilerWord,
    project_native_compiler_words,
};
use crate::native_compiler_words::NativeCompilerWords;
use tcl_dialect::TclVersion;
use tcl_lexer::ExecutablePart;

/// Physical namespace selected by the admitted compiler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeNamespaceBindingKind {
    /// NSUPVAR retains the registered root namespace operand.
    Global,
    /// VARIABLE selects the actual procedure's namespace at runtime.
    Variable,
    /// C8.5's monolithic namespace compiler retains an explicit namespace word.
    Upvar,
}

/// An ordered original variable binding and optional scalar store.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeNamespaceBinding {
    /// Original evaluated name or exact parser-expanded literal member.
    pub name: NativeCompilerWordOperand,
    /// Counted compile-time local name, without reparsing a display name.
    pub local: Vec<u8>,
    /// Original value evaluated after this binding, before the next binding.
    pub value: Option<NativeCompilerWordOperand>,
}

/// Native compiler visits made before a later name can decline compilation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NativeNamespaceBindingVisit {
    /// Register the original compiler's implicit constant before its name loop.
    Literal(Vec<u8>),
    /// Register a compiled local at this point in the selected compiler visits.
    DeclareLocal(Vec<u8>),
    /// Compile this original name or value word and its reached substitutions.
    Word(NativeCompilerWordOperand),
}

/// C compiler outcome, independent of raw hook and original handler identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeNamespaceBindingOutcome {
    /// Emit the complete retained bind/store recipe.
    Inline,
    /// Compiler declines; prefix declarations/visits remain preparation facts.
    Generic,
    /// The actual compiler environment has not been established.
    Unknown,
}

/// Original namespace-binding compiler recipe and its preparation residual.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeNamespaceBindingCompilation {
    /// Root, current or original explicit namespace binding.
    pub kind: NativeNamespaceBindingKind,
    /// Original explicit namespace operand, evaluated before the first target.
    pub namespace: Option<NativeCompilerWordOperand>,
    /// Selected compiler outcome; partial bindings cannot execute when Generic.
    pub outcome: NativeNamespaceBindingOutcome,
    /// Accepted prefix bindings, in the actual compiler's order.
    pub bindings: Vec<NativeNamespaceBinding>,
    /// Ordered compiler visits, retained even when a later name declines.
    pub visits: Vec<NativeNamespaceBindingVisit>,
}

/// Missing lexical evidence remains a compiler obligation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeNamespaceBindingUnavailable {
    /// The C parser cannot project the supplied retained source.
    Projection(NativeCompilerProjectionUnavailable),
    /// Operand indices do not identify a nonempty command vector.
    OperandGeometry,
    /// A selected source escape could not publish its original token extent.
    SourceEscape,
}

/// Select global/variable compilation from original native compiler words.
///
/// This consumes parser words, compiler mode/frame and release only. Callers
/// must independently authenticate the original registered compiler/handler,
/// assign actual LVT indices and retain emitted name objects at runtime.
///
/// # Errors
/// Returns explicit source/geometry obligations instead of guessing a tail.
pub fn compile_native_namespace_bindings(
    words: &NativeCompilerWords<'_>,
    operand_from: usize,
    version: TclVersion,
    context: NativeCompilationContext,
    kind: NativeNamespaceBindingKind,
) -> Result<NativeNamespaceBindingCompilation, NativeNamespaceBindingUnavailable> {
    use NativeNamespaceBindingOutcome as Outcome;
    if kind == NativeNamespaceBindingKind::Upvar {
        if version >= TclVersion::V8_6 {
            return crate::native_namespace_upvar_compilation::compile_native_namespace_upvar_worker(
                words, operand_from, version, context,
            );
        }
        return crate::native_namespace_upvar_compilation::compile_native_namespace_upvar(
            words,
            operand_from,
            version,
            context,
        );
    }
    let mut result = NativeNamespaceBindingCompilation {
        kind,
        namespace: None,
        outcome: Outcome::Generic,
        bindings: Vec::new(),
        visits: Vec::new(),
    };
    if operand_from == 0 || operand_from > words.original_words().len() {
        return Err(NativeNamespaceBindingUnavailable::OperandGeometry);
    }
    if version < TclVersion::V8_5
        || context.mode == NativeCompilationMode::Direct
        || context.frame == NativeCompilationFrame::ScriptCode
    {
        return Ok(result);
    }
    if context.mode != NativeCompilationMode::BytecodeObject
        || context.frame != NativeCompilationFrame::ProcedureCode
    {
        result.outcome = Outcome::Unknown;
        return Ok(result);
    }
    // These compilers do not receive an unresolved EXPAND_WORD. Parser-known
    // expansions are projected before the compiler's token/count checks.
    let projected = project_native_compiler_words(words, version)
        .map_err(NativeNamespaceBindingUnavailable::Projection)?;
    let operands: Vec<_> = projected
        .iter()
        .filter(|word| {
            let original = match word.operand {
                NativeCompilerWordOperand::Original(index) => index,
                NativeCompilerWordOperand::LiteralExpansion { original_word, .. } => original_word,
            };
            original >= operand_from
        })
        .collect();
    if operands.is_empty()
        || projected
            .iter()
            .any(|word| word.shape == NativeCompilationWordShape::Expanded)
        || (version == TclVersion::V9_1 && u32::try_from(projected.len()).is_err())
    {
        return Ok(result);
    }
    let stride = if kind == NativeNamespaceBindingKind::Global {
        1
    } else {
        2
    };
    if kind == NativeNamespaceBindingKind::Global {
        result
            .visits
            .push(NativeNamespaceBindingVisit::Literal(b"::".to_vec()));
    }
    for position in (0..operands.len()).step_by(stride) {
        let name = operands[position];
        let Some(local) = compiler_local_tail(words, name, version)? else {
            return Ok(result);
        };
        let value = if kind == NativeNamespaceBindingKind::Variable {
            operands.get(position + 1).map(|word| word.operand.clone())
        } else {
            None
        };
        result
            .visits
            .push(NativeNamespaceBindingVisit::DeclareLocal(local.clone()));
        result
            .visits
            .push(NativeNamespaceBindingVisit::Word(name.operand.clone()));
        if let Some(value) = &value {
            result
                .visits
                .push(NativeNamespaceBindingVisit::Word(value.clone()));
        }
        result.bindings.push(NativeNamespaceBinding {
            name: name.operand.clone(),
            local,
            value,
        });
    }
    result.outcome = Outcome::Inline;
    Ok(result)
}

/// Shape-only compatibility queries cannot establish a substituted final TEXT
/// token. Their known scalar tails share the original binding owner's rule.
pub(crate) fn namespace_binding_shape_outcome(
    words: crate::InvocationWords<'_>,
    shapes: &[NativeCompilationWordShape],
    version: TclVersion,
    context: NativeCompilationContext,
    kind: NativeNamespaceBindingKind,
) -> NativeNamespaceBindingOutcome {
    use NativeNamespaceBindingOutcome as Outcome;
    if version < TclVersion::V8_5
        || context.mode == NativeCompilationMode::Direct
        || context.frame == NativeCompilationFrame::ScriptCode
        || shapes.is_empty()
    {
        return Outcome::Generic;
    }
    if context.mode != NativeCompilationMode::BytecodeObject
        || context.frame != NativeCompilationFrame::ProcedureCode
        || words.arguments().exact_argv_len() != Some(shapes.len())
    {
        return Outcome::Unknown;
    }
    let stride = if kind == NativeNamespaceBindingKind::Global {
        1
    } else {
        2
    };
    for index in (0..shapes.len()).step_by(stride) {
        if !matches!(
            shapes[index],
            NativeCompilationWordShape::Literal
                | NativeCompilationWordShape::QuotedLiteral
                | NativeCompilationWordShape::BracedLiteral
                | NativeCompilationWordShape::BackslashLiteral
        ) {
            return Outcome::Unknown;
        }
        let Some(name) = words.arguments().literal_at(index) else {
            return Outcome::Unknown;
        };
        if scalar_tail(name.as_bytes(), true).is_none() {
            return Outcome::Generic;
        }
    }
    Outcome::Inline
}

fn compiler_local_tail(
    words: &NativeCompilerWords<'_>,
    word: &NativeProjectedCompilerWord,
    version: TclVersion,
) -> Result<Option<Vec<u8>>, NativeNamespaceBindingUnavailable> {
    if let Some(value) = &word.literal {
        return Ok(scalar_tail(value, true));
    }
    // C8.5's TclWordKnownAtCompileTime probe rejects a standalone TEXT token;
    // C8.6+ explicitly reads the original final TEXT component.
    if version == TclVersion::V8_5 {
        return Ok(None);
    }
    let NativeCompilerWordOperand::Original(index) = word.operand else {
        return Ok(None);
    };
    let original = &words.original_words()[index];
    let arena = original.executable_parts();
    let Some(mut component) = arena.list(arena.root()).last() else {
        return Ok(None);
    };
    // The C parser counts VARIABLE children in the enclosing WORD. Its final
    // token can be the name TEXT or a final index component, not the VARIABLE.
    loop {
        match component.part {
            ExecutablePart::Variable { name, index: None } => {
                let raw = arena
                    .bytes(name)
                    .ok_or(NativeNamespaceBindingUnavailable::OperandGeometry)?;
                let value =
                    tcl_syntax::backslash::source_literal_bytes(raw, original.image().channel());
                return Ok(scalar_tail(&value, false));
            }
            ExecutablePart::Variable {
                index: Some(index), ..
            } => {
                let Some(last) = arena.list(index).last() else {
                    // Tcl_ParseVarName emits an empty TEXT token for an empty index.
                    return Ok(Some(Vec::new()));
                };
                component = last;
            }
            ExecutablePart::Text(_) => break,
            _ => return Ok(None),
        }
    }
    let raw = arena
        .bytes(component.span)
        .ok_or(NativeNamespaceBindingUnavailable::OperandGeometry)?;
    // A decoded arena text can contain several native TEXT/BS components.
    // Select the suffix after the final original BS token with the shared
    // lexical escape extent, rather than treating decoded bytes as one token.
    let mut cursor = 0;
    let mut suffix = 0;
    while cursor < raw.len() {
        if raw[cursor] == b'\\' {
            let end = tcl_syntax::backslash::native_source_escape_channel_in(
                raw,
                cursor,
                original.image().channel(),
                original.config().escapes,
                words.source_protocol(),
            )
            .map_err(|_| NativeNamespaceBindingUnavailable::SourceEscape)?
            .end;
            if end <= cursor {
                return Err(NativeNamespaceBindingUnavailable::SourceEscape);
            }
            cursor = end;
            suffix = end;
        } else {
            cursor += 1;
        }
    }
    if suffix == raw.len() {
        return Ok(None);
    }
    let value =
        tcl_syntax::backslash::source_literal_bytes(&raw[suffix..], original.image().channel());
    Ok(scalar_tail(&value, false))
}

fn scalar_tail(value: &[u8], complete: bool) -> Option<Vec<u8>> {
    if value.last() == Some(&b')') {
        return None;
    }
    let separator = value.windows(2).rposition(|bytes| bytes == b"::");
    if !complete && !value.is_empty() && separator.is_none() {
        return None;
    }
    Some(value[separator.map_or(0, |index| index + 2)..].to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_lexer::{LexerConfig, SourceImage, Span, native_script_words_in};
    use tcl_syntax::native_string::NativeStringProtocol;

    const BODIES: [&[u8]; 20] = [
        b"variable v",
        b"variable {}",
        b"variable :::",
        b"variable {a)}",
        b"variable v $val w [set x 3]",
        b"variable v 1 {a)} 2",
        b"variable ${ns}::v",
        b"variable ${ns}v",
        b"variable ${ns}::[set x 1]",
        b"variable ${ns}::v 1 ${ns}v 2",
        b"variable {*}{v 1}",
        b"variable",
        b"global v",
        b"global {}",
        b"global :::",
        b"global {a)}",
        b"global ${ns}::v",
        b"global ${ns}v",
        b"global v {a)}",
        b"global {*}{v w}",
    ];

    fn context() -> NativeCompilationContext {
        NativeCompilationContext {
            mode: NativeCompilationMode::BytecodeObject,
            frame: NativeCompilationFrame::ProcedureCode,
            ..Default::default()
        }
    }

    fn hex(bytes: &[u8]) -> String {
        use std::fmt::Write;
        bytes.iter().fold(String::new(), |mut text, byte| {
            write!(text, "{byte:02x}").unwrap();
            text
        })
    }

    #[test]
    fn namespace_binding_recipes_match_all_100_original_native_windows() {
        for (version, fixture) in [
            (
                TclVersion::V8_4,
                include_str!("../tests/data/native_namespace_bindings/8.4.20.tsv"),
            ),
            (
                TclVersion::V8_5,
                include_str!("../tests/data/native_namespace_bindings/8.5.19.tsv"),
            ),
            (
                TclVersion::V8_6,
                include_str!("../tests/data/native_namespace_bindings/8.6.18.tsv"),
            ),
            (
                TclVersion::V9_0,
                include_str!("../tests/data/native_namespace_bindings/9.0.4.tsv"),
            ),
            (
                TclVersion::V9_1,
                include_str!("../tests/data/native_namespace_bindings/9.1.0.tsv"),
            ),
        ] {
            let profile =
                tcl_dialect::DialectProfile::find(&format!("tcl{}", version.version_string()))
                    .unwrap();
            let mut count = 0;
            for line in fixture.lines().skip(1) {
                let columns: Vec<_> = line.split('\t').collect();
                let index: usize = columns[0].parse().unwrap();
                let variable: i32 = columns[2].parse().unwrap();
                let global: i32 = columns[3].parse().unwrap();
                let image = SourceImage::native(BODIES[index]);
                let parsed = native_script_words_in(
                    image.clone(),
                    Span::new(0, u32::try_from(image.len()).unwrap()),
                    LexerConfig::from_grammar(profile.grammar),
                );
                count += 1;
                if variable < 0 {
                    assert_eq!(version, TclVersion::V8_4);
                    let rejected = parsed.unwrap();
                    assert!(
                        rejected.fatal_tail.is_some(),
                        "original C8.4 parser rejects adjacent expansion braces"
                    );
                    assert!(rejected.commands.is_empty());
                    continue;
                }
                let original = parsed.unwrap().commands.remove(0).words;
                let words =
                    NativeCompilerWords::capture(&original, NativeStringProtocol::C(version))
                        .unwrap();
                let kind = if index < 12 {
                    NativeNamespaceBindingKind::Variable
                } else {
                    NativeNamespaceBindingKind::Global
                };
                let recipe =
                    compile_native_namespace_bindings(&words, 1, version, context(), kind).unwrap();
                let native_count = usize::try_from(variable + global).unwrap();
                assert_eq!(
                    if recipe.outcome == NativeNamespaceBindingOutcome::Inline {
                        recipe.bindings.len()
                    } else {
                        0
                    },
                    native_count,
                    "{version:?} case {index}",
                );
                let declarations: Vec<_> = recipe
                    .visits
                    .iter()
                    .filter_map(|visit| match visit {
                        NativeNamespaceBindingVisit::DeclareLocal(name) => Some(hex(name)),
                        NativeNamespaceBindingVisit::Word(_)
                        | NativeNamespaceBindingVisit::Literal(_) => None,
                    })
                    .collect();
                // The child [set x ...] compiler independently declares x.
                let native_names: Vec<_> = columns[4]
                    .split(',')
                    .skip(2)
                    .filter(|name| *name != "78")
                    .collect();
                assert_eq!(declarations, native_names, "{version:?} case {index}");
                assert_eq!(
                    columns[1],
                    if version == TclVersion::V8_4 {
                        "0"
                    } else {
                        "1"
                    }
                );
            }
            assert_eq!(count, 20);
        }
    }

    #[test]
    fn namespace_dynamic_tails_follow_original_variable_token_children() {
        let controls: [(&[u8], Option<&[u8]>); 8] = [
            (b"global $::name", Some(b"name")),
            (b"global ${::name}", Some(b"name")),
            (b"global $a()", Some(b"")),
            (b"global $a(::tail)", Some(b"tail")),
            (b"global $a($::name)", Some(b"name")),
            (b"global $a([foo])", None),
            (b"global $a(\\x61)", None),
            (b"global $a($::name)::tail", Some(b"tail")),
        ];
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let profile =
                tcl_dialect::DialectProfile::find(&format!("tcl{}", version.version_string()))
                    .unwrap();
            for (source, expected) in controls {
                let image = SourceImage::native(source);
                let original = native_script_words_in(
                    image.clone(),
                    Span::new(0, u32::try_from(image.len()).unwrap()),
                    LexerConfig::from_grammar(profile.grammar),
                )
                .unwrap()
                .commands
                .remove(0)
                .words;
                let words =
                    NativeCompilerWords::capture(&original, NativeStringProtocol::C(version))
                        .unwrap();
                let recipe = compile_native_namespace_bindings(
                    &words,
                    1,
                    version,
                    context(),
                    NativeNamespaceBindingKind::Global,
                )
                .unwrap();
                let expected = (version >= TclVersion::V8_6).then_some(expected).flatten();
                assert_eq!(
                    recipe
                        .bindings
                        .first()
                        .map(|binding| binding.local.as_slice()),
                    expected,
                    "{version:?}: {source:?}"
                );
                assert_eq!(
                    recipe.outcome == NativeNamespaceBindingOutcome::Inline,
                    expected.is_some(),
                    "{version:?}: {source:?}"
                );
            }
        }
    }

    #[test]
    fn unknown_and_script_frames_do_not_grant_namespace_binding_instructions() {
        let version = TclVersion::V9_1;
        let image = SourceImage::native(b"variable v $value".as_slice());
        let profile = tcl_dialect::DialectProfile::find("tcl9.1").unwrap();
        let original = native_script_words_in(
            image.clone(),
            Span::new(0, u32::try_from(image.len()).unwrap()),
            LexerConfig::from_grammar(profile.grammar),
        )
        .unwrap()
        .commands
        .remove(0)
        .words;
        let words =
            NativeCompilerWords::capture(&original, NativeStringProtocol::C(version)).unwrap();
        for (frame, outcome) in [
            (
                NativeCompilationFrame::Unknown,
                NativeNamespaceBindingOutcome::Unknown,
            ),
            (
                NativeCompilationFrame::ScriptCode,
                NativeNamespaceBindingOutcome::Generic,
            ),
        ] {
            let recipe = compile_native_namespace_bindings(
                &words,
                1,
                version,
                NativeCompilationContext { frame, ..context() },
                NativeNamespaceBindingKind::Variable,
            )
            .unwrap();
            assert_eq!(recipe.outcome, outcome);
            assert!(recipe.bindings.is_empty());
            assert!(recipe.visits.is_empty());
        }
    }
}
