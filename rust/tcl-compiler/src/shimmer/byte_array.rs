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

//! Byte-array corruption detection (S110).
//!
//! A **correctness** shimmer, distinct from the S100/S101/S102 *performance*
//! family. Tcl byte arrays (binary data) and character strings are different
//! internal representations. When a byte array is forced through a
//! character-string operation and then written back to a byte sink, every byte
//! `>= 0x80` is silently re-encoded (latin-1 decode → UTF-8 encode, or pushed
//! out of the `0..255` range by case folding), corrupting the data. In iRules
//! this is the canonical `*::payload replace` rewrite bug
//! ([F5 K22406348](https://my.f5.com/manage/s/article/K22406348)); in plain
//! Tcl it is `binary format` → `string …` → byte sink.
//!
//! The check is a small forward dataflow over the SSA graph tracking *byte
//! provenance* per value:
//!
//! - [`ByteProv::Binary`] — the value currently has a byte-array intrep (safe
//!   to write to a byte sink). Sources: `*::payload` getters
//!   ([`CommandSpec::byte_array_payload`]) and any command / subcommand whose
//!   declared return type is `TclType::ByteArray`.
//! - [`ByteProv::Damaged`] — a binary-sourced value that has since been coerced
//!   to a character string (interpolation, a [`ByteArrayEffect::Coerces`] /
//!   [`ByteArrayEffect::CaseFolds`] transform, `expr`, …). Writing it to a
//!   byte sink corrupts it.
//!
//! A [`ByteArrayEffect::Rebinarifies`] statement (`binary scan $v …`)
//! re-installs the byte-array rep on its value operand in place — the
//! documented fix — and clears [`ByteProv::Damaged`].
//!
//! Every command-specific fact — which forms are sources, sinks, transforms,
//! encoders, or re-binarifiers, and the labels used in messages — is registry
//! data ([`ByteArrayEffect`], return types, [`BytePayloadSpec`]); this pass
//! never matches a command or subcommand by name.
//!
//! See `docs/design/compiler/byte-array-corruption.md`.
//!
//! [`CommandSpec::byte_array_payload`]: tcl_registry::CommandSpec::byte_array_payload

use std::collections::HashMap;
use tcl_core_types::DiagCode;

use tcl_lexer::Span;
use tcl_registry::{ByteArrayEffect, BytePayloadSpec, CommandRegistry, TclType};

use crate::cfg::{BlockId, Function as CfgFunction};
use crate::ir::{Statement, WordExpr, WordPart};
use crate::naming::normalise_var_name;
use crate::registry_invocation::NormalRepresentationInvocation;
use crate::sccp::cfg_order;
use crate::ssa::{SsaFunction, Symbol, ValueKey};

use std::collections::HashSet;

use super::ShimmerWarning;

/// Byte-array provenance of a tracked value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ByteProv {
    /// Currently a byte array — safe at a byte sink.
    Binary,
    /// Binary-sourced but coerced to a character string.
    Damaged,
}

/// Provenance of a value plus the spans/labels for the diagnostic.
#[derive(Debug, Clone)]
struct ByteProvInfo {
    state: ByteProv,
    /// Where the binary data originated (`None` when sourced inline at a use).
    source_range: Option<Span>,
    source_label: String,
    /// Where the value was coerced to a character string (`None` until coerced).
    coercion_range: Option<Span>,
    coercion_label: String,
}

impl ByteProvInfo {
    /// A freshly-sourced byte array (no coercion yet).
    fn binary(source_range: Option<Span>, source_label: String) -> Self {
        Self {
            state: ByteProv::Binary,
            source_range,
            source_label,
            coercion_range: None,
            coercion_label: String::new(),
        }
    }

    /// A damaged value derived from `origin`, coerced at `coercion`.
    fn damaged(origin: &Self, coercion_range: Option<Span>, coercion_label: String) -> Self {
        Self {
            state: ByteProv::Damaged,
            source_range: origin.source_range,
            source_label: origin.source_label.clone(),
            coercion_range,
            coercion_label,
        }
    }
}

/// Registry-declared S110 classification of one `[cmd arg…]` form, resolved
/// through the command spec (and its subcommand when the first arg names one).
/// No command or subcommand names are matched here: the effect, the
/// byte-array-source flag, the operand window, and the diagnostic label are
/// all spec data.
struct CmdEffect {
    /// Effect declared on the resolved subcommand or on the bare command.
    effect: ByteArrayEffect,
    /// Whether the form's declared return type is `TclType::ByteArray` — a
    /// binary source (`binary format` / `binary decode` / `encoding
    /// convertto`).
    returns_byte_array: bool,
    /// Index of the first value-operand argument: `1` when the classification
    /// came from a subcommand word, `0` for a bare command.
    operand_start: usize,
    /// Spec-derived diagnostic label — `cmd` or `cmd subcommand` (the
    /// subcommand's canonical spec name, so abbreviations normalise).
    label: String,
}

impl CmdEffect {
    fn selected(invocation: &NormalRepresentationInvocation) -> Self {
        Self {
            effect: invocation.byte_array_effect(),
            returns_byte_array: invocation.returns_byte_array(),
            operand_start: invocation.argument_offset(),
            label: invocation.diagnostic_label(),
        }
    }

    /// An inert classification labelled with the bare command word.
    #[cfg(test)]
    fn inert(cmd: &str) -> Self {
        Self {
            effect: ByteArrayEffect::None,
            returns_byte_array: false,
            operand_start: 0,
            label: cmd.to_owned(),
        }
    }
}

/// Resolve the registry's S110 classification of `[cmd args…]`. A
/// subcommand-typed command (`string`, `binary`, `encoding`) carries the
/// classification on the resolved subcommand; a bare command carries it at the
/// command level; an unknown command (or an unresolved first word, e.g. a
/// `TCP::payload <size>` getter) is inert.
#[cfg(test)]
fn resolve_cmd_effect(registry: &CommandRegistry, cmd: &str, args: &[String]) -> CmdEffect {
    let Some(spec) = registry.get(cmd) else {
        return CmdEffect::inert(cmd);
    };
    // Ask the registry what *this* call returns rather than reading
    // `return_type` raw, so a per-form result can never be classified here
    // differently from how SSA type propagation and the taint sanitiser
    // classify it.  No byte-array command declares a
    // return-type hook today; this keeps that entry point honest if one
    // ever does.
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let returns_byte_array = spec.return_type_for_call(&arg_refs) == Some(TclType::ByteArray);
    if spec.subcommands.is_empty() {
        return CmdEffect {
            effect: spec.byte_array_effect,
            returns_byte_array,
            operand_start: 0,
            label: cmd.to_owned(),
        };
    }
    match args.first().and_then(|w| spec.resolve_subcommand(w)) {
        Some(sub) => CmdEffect {
            effect: sub.byte_array_effect,
            returns_byte_array,
            operand_start: 1,
            label: format!("{cmd} {}", sub.name),
        },
        None => CmdEffect::inert(cmd),
    }
}

/// True when `[cmd args]` reads raw payload bytes (the getter form of a
/// registry `*::payload` byte command).
#[cfg(test)]
fn is_payload_getter(
    layouts: &HashMap<&'static str, BytePayloadSpec>,
    cmd: &str,
    args: &[String],
) -> bool {
    layouts.contains_key(cmd) && BytePayloadSpec::is_getter_call(args.first().map(String::as_str))
}

fn with_payload_arguments<T>(
    invocation: &NormalRepresentationInvocation,
    select: impl FnOnce(tcl_registry::InvocationArguments<'_>) -> T,
) -> T {
    let values: Vec<_> = (0..invocation.argument_count())
        .map(|index| invocation.argument_literal(index))
        .collect();
    let words: Vec<_> = values
        .iter()
        .enumerate()
        .map(
            |(index, value)| match invocation.effective_words().words.get(index + 1) {
                Some(WordExpr::Expand { .. }) => tcl_registry::InvocationWord::Expanded,
                Some(WordExpr::Opaque { .. }) => tcl_registry::InvocationWord::Opaque,
                _ => value.as_deref().map_or(
                    tcl_registry::InvocationWord::Dynamic,
                    tcl_registry::InvocationWord::Literal,
                ),
            },
        )
        .collect();
    select(tcl_registry::InvocationArguments::structured(&words))
}

fn payload_is_getter(invocation: &NormalRepresentationInvocation) -> bool {
    with_payload_arguments(invocation, BytePayloadSpec::is_getter_invocation) == Some(true)
}

fn payload_data_index(
    layout: BytePayloadSpec,
    invocation: &NormalRepresentationInvocation,
) -> Option<usize> {
    with_payload_arguments(invocation, |arguments| {
        layout.replace_data_arg_for_invocation(arguments)
    })
}

/// Join byte provenance over several inputs — DAMAGED dominates BINARY
/// (may-corrupt); the first non-empty source of each state is kept for the
/// diagnostic.
fn join_prov(infos: impl IntoIterator<Item = Option<ByteProvInfo>>) -> Option<ByteProvInfo> {
    let mut damaged: Option<ByteProvInfo> = None;
    let mut binary: Option<ByteProvInfo> = None;
    for info in infos.into_iter().flatten() {
        match info.state {
            ByteProv::Damaged if damaged.is_none() => damaged = Some(info),
            ByteProv::Binary if binary.is_none() => binary = Some(info),
            _ => {}
        }
    }
    damaged.or(binary)
}

/// Build the S110 warning, with related spans pointing at the binary source
/// and the coercion site.
fn byte_warning(
    span: Span,
    variable: &str,
    info: &ByteProvInfo,
    message: String,
) -> ShimmerWarning {
    let mut related: Vec<(Span, String)> = Vec::new();
    if let Some(src) = info.source_range {
        related.push((src, format!("binary data from {} here", info.source_label)));
    }
    if let Some(coerce) = info.coercion_range
        && Some(coerce) != info.source_range
    {
        let label = if info.coercion_label.is_empty() {
            "string operation"
        } else {
            &info.coercion_label
        };
        related.push((
            coerce,
            format!("treated as a character string by {label} here"),
        ));
    }
    let command = if info.coercion_label.is_empty() {
        info.source_label.clone()
    } else {
        info.coercion_label.clone()
    };
    ShimmerWarning {
        span,
        variable: variable.to_owned(),
        from_type: TclType::ByteArray,
        to_type: TclType::String,
        command,
        in_loop: false,
        code: DiagCode::S110,
        message,
        related,
    }
}

/// Forward byte-provenance dataflow state for one function.
struct ByteCorruption<'a> {
    context: super::ShimmerContext<'a>,
    prov: HashMap<ValueKey, ByteProvInfo>,
    warnings: Vec<ShimmerWarning>,
}

impl<'a> ByteCorruption<'a> {
    fn new(
        context: super::ShimmerContext<'a>,
        _payload_layouts: &HashMap<&'static str, BytePayloadSpec>,
    ) -> Self {
        Self {
            context,
            prov: HashMap::new(),
            warnings: Vec::new(),
        }
    }

    /// Run the dataflow over the function's executable blocks in CFG order,
    /// joining phis first, and return the accumulated warnings.
    fn run(
        mut self,
        cfg: &CfgFunction,
        ssa: &SsaFunction,
        executable_blocks: &HashSet<BlockId>,
    ) -> Vec<ShimmerWarning> {
        for block_id in cfg_order(cfg) {
            if !executable_blocks.contains(&block_id) {
                continue;
            }
            let Some(ssa_block) = ssa.blocks.get(&block_id) else {
                continue;
            };

            for phi in &ssa_block.phis {
                let joined = join_prov(
                    phi.incoming
                        .values()
                        .filter(|&&v| v > 0)
                        .map(|&v| self.prov.get(&(phi.name, v)).cloned()),
                );
                if let Some(joined) = joined {
                    self.prov.insert((phi.name, phi.version), joined);
                }
            }

            for (index, ss) in ssa_block.statements.iter().enumerate() {
                if let Some(versions) = ssa
                    .value_clobbers
                    .get(&block_id)
                    .and_then(|markers| markers.get(&index))
                {
                    for (&symbol, &(prior, fresh)) in versions {
                        if let Some(provenance) = self.prov.get(&(symbol, prior)).cloned() {
                            self.prov.insert((symbol, fresh), provenance);
                        }
                    }
                }
                let source = crate::ssa::SsaSourceView::at_statement(ssa, block_id, index);
                match &ss.statement {
                    Statement::AssignValue { name, span, .. } => {
                        self.track_assign_value(name, *span, &ss.defs, source);
                    }
                    Statement::AssignExpr { name, span, .. } => {
                        self.track_assign_expr(name, *span, &ss.defs, source);
                    }
                    Statement::Call { span, .. } => {
                        self.track_call(*span, &ss.defs, &ss.uses, source);
                    }
                    _ => {}
                }
            }
        }
        self.warnings
    }

    fn selected_invocation(
        &self,
        ssa: crate::ssa::SsaSourceView<'_>,
    ) -> Option<NormalRepresentationInvocation> {
        self.context.invocation(ssa.source_tokens()?)
    }

    fn nested_invocation(
        &self,
        word: &WordExpr,
        ssa: crate::ssa::SsaSourceView<'_>,
    ) -> Option<NormalRepresentationInvocation> {
        let parent = ssa.source_tokens()?;
        let config = self.context.config();
        let mut tokens = crate::word_subst::whole_word_command_tokens(word, config)?;
        tokens.inherit_nested_bindings(parent);
        self.context.invocation(&tokens)
    }

    fn operand_prov(
        &self,
        invocation: &NormalRepresentationInvocation,
        ssa: crate::ssa::SsaSourceView<'_>,
        depth: u32,
    ) -> Option<ByteProvInfo> {
        if invocation.byte_array_effect() == ByteArrayEffect::Encodes {
            return self.arg_byte_prov(&invocation.encoded_value_word()?, ssa, depth);
        }
        let start = invocation.argument_offset().saturating_add(1);
        join_prov(
            invocation
                .effective_words()
                .words
                .get(start..)?
                .iter()
                .map(|word| self.arg_byte_prov(word, ssa, depth)),
        )
    }

    /// Object provenance follows the exact executed read, including reads
    /// before and after an embedded alias or contents mutation.
    fn arg_byte_prov(
        &self,
        word: &WordExpr,
        ssa: crate::ssa::SsaSourceView<'_>,
        depth: u32,
    ) -> Option<ByteProvInfo> {
        if crate::depth_guard::MAX_BRACKET_TEXT_DEPTH.exceeded(depth) {
            return None;
        }
        if word.sole_variable_substitution().is_some() {
            return self.read_byte_provenance(word, ssa);
        }
        if let Some(invocation) = self.nested_invocation(word, ssa) {
            let ce = CmdEffect::selected(&invocation);
            if invocation.byte_array_payload().is_some() && payload_is_getter(&invocation) {
                return Some(ByteProvInfo::binary(None, ce.label));
            }
            if ce.effect != ByteArrayEffect::Encodes && ce.returns_byte_array {
                return Some(ByteProvInfo::binary(None, ce.label));
            }
            let operand = self.operand_prov(&invocation, ssa, depth + 1);
            return match ce.effect {
                ByteArrayEffect::Encodes => Some(match operand {
                    Some(op) => ByteProvInfo::damaged(&op, None, ce.label),
                    None => ByteProvInfo::binary(None, ce.label),
                }),
                ByteArrayEffect::Transparent => operand,
                ByteArrayEffect::CaseFolds
                | ByteArrayEffect::Coerces
                | ByteArrayEffect::None
                | ByteArrayEffect::Rebinarifies { .. } => {
                    operand.map(|inner| ByteProvInfo::damaged(&inner, None, ce.label))
                }
            };
        }
        let WordExpr::Template { parts, .. } = word else {
            return None;
        };
        let joined = join_prov(parts.iter().map(|part| {
            let expression = match part {
                WordPart::Variable { spelling, source } => WordExpr::Variable {
                    spelling: spelling.clone(),
                    source: source.clone(),
                },
                WordPart::CommandSubstitution { spelling, source } => {
                    WordExpr::CommandSubstitution {
                        spelling: spelling.clone(),
                        source: source.clone(),
                    }
                }
                WordPart::Text { .. } | WordPart::Opaque { .. } => return None,
            };
            self.arg_byte_prov(&expression, ssa, depth + 1)
        }));
        joined.map(|info| ByteProvInfo::damaged(&info, None, "interpolation".to_owned()))
    }

    /// A positioned read may have several represented reaching writes even
    /// when no single SSA version describes its contents. The shared contents
    /// owner proves their physical cells; display names never select producers.
    fn read_byte_provenance(
        &self,
        word: &WordExpr,
        source: crate::ssa::SsaSourceView<'_>,
    ) -> Option<ByteProvInfo> {
        let read = source.read_word(word)?;
        if let Some(version) = read.version {
            return self.prov.get(&(read.symbol, version)).cloned();
        }
        let contents = source.read_word_contents(word, self.context.registry())?;
        if contents.unknown_residual || contents.includes_incoming {
            return None;
        }
        join_prov(contents.writes.iter().map(|&(block, index)| {
            let statement = source
                .function()
                .blocks
                .get(&block)?
                .statements
                .get(index)?;
            let version = *statement.defs.get(&read.symbol)?;
            self.prov.get(&(read.symbol, version)).cloned()
        }))
    }

    fn track_assign_value(
        &mut self,
        name: &str,
        span: Span,
        defs: &HashMap<Symbol, u32>,
        ssa: crate::ssa::SsaSourceView<'_>,
    ) {
        let Some(assignment) = self
            .selected_invocation(ssa)
            .and_then(|invocation| invocation.value_assignment())
        else {
            return;
        };
        self.track_stored_value(name, &assignment.value, span, defs, ssa);
    }

    fn track_stored_value(
        &mut self,
        name: &str,
        word: &WordExpr,
        span: Span,
        defs: &HashMap<Symbol, u32>,
        ssa: crate::ssa::SsaSourceView<'_>,
    ) {
        let nm = normalise_var_name(name);
        let Some(sym) = ssa.symbol(nm) else {
            return;
        };
        let Some(&version) = defs.get(&sym) else {
            return;
        };
        let key = (sym, version);
        if let Some(nested) = self.nested_invocation(word, ssa) {
            let ce = CmdEffect::selected(&nested);
            let operand = self.operand_prov(&nested, ssa, 1);
            if ce.effect == ByteArrayEffect::Encodes {
                if let Some(op) = operand {
                    let message = format!(
                        "Byte-array corruption: '{}' on binary data from {} double-encodes it — \
                         the bytes are reinterpreted as characters and re-encoded. Decode with \
                         'encoding convertfrom' or keep it a byte array (S110)",
                        ce.label, op.source_label
                    );
                    self.warnings.push(byte_warning(span, nm, &op, message));
                }
                self.prov
                    .insert(key, ByteProvInfo::binary(Some(span), ce.label));
                return;
            }
            if nested.byte_array_payload().is_some() && payload_is_getter(&nested)
                || ce.returns_byte_array
            {
                self.prov
                    .insert(key, ByteProvInfo::binary(Some(span), ce.label));
                return;
            }
            self.apply_byte_array_effect(&ce, key, span, nm, operand);
            return;
        }
        if let Some(mut info) = self.arg_byte_prov(word, ssa, 0) {
            if info.coercion_label == "interpolation" {
                info.coercion_range = Some(span);
            }
            self.prov.insert(key, info);
        }
    }

    /// Apply the registry-declared [`ByteArrayEffect`] of a resolved form to
    /// the value assigned at `key`: a case-fold warns and marks it damaged, a
    /// transparent op propagates the operand's provenance unchanged, a coercing
    /// op marks it damaged, and the rest are inert in an assignment.
    fn apply_byte_array_effect(
        &mut self,
        ce: &CmdEffect,
        key: ValueKey,
        span: Span,
        nm: &str,
        operand: Option<ByteProvInfo>,
    ) {
        match ce.effect {
            // Case-folding reinterprets the bytes as Unicode code points,
            // mangling every byte >= 0x80 directly (with or without a sink).
            ByteArrayEffect::CaseFolds => {
                if let Some(op) = operand {
                    let msg = format!(
                        "Byte-array corruption: '{label}' on binary data from {src} \
                         reinterprets bytes as Unicode characters, mangling every byte \
                         >= 0x80 (S110)",
                        label = ce.label,
                        src = op.source_label,
                    );
                    self.warnings.push(byte_warning(span, nm, &op, msg));
                    self.prov.insert(
                        key,
                        ByteProvInfo::damaged(&op, Some(span), ce.label.clone()),
                    );
                }
            }
            // Transparent ops keep the byte-array representation, so
            // provenance passes through unchanged — a binary operand stays
            // binary (byte-exact at a sink), an already-damaged operand stays
            // damaged.
            ByteArrayEffect::Transparent => {
                if let Some(op) = operand {
                    self.prov.insert(key, op);
                }
            }
            // Coercing string-builders derive a character string from a
            // (possibly binary) operand; a byte sink then re-encodes it.
            ByteArrayEffect::Coerces => {
                if let Some(derived) = operand {
                    self.prov.insert(
                        key,
                        ByteProvInfo::damaged(&derived, Some(span), ce.label.clone()),
                    );
                }
            }
            // Inert in an assignment: not a value transform (`None`), an
            // in-place statement-level fix (`Rebinarifies`), or dispatched by
            // the caller before this point (`Encodes`).
            ByteArrayEffect::None
            | ByteArrayEffect::Rebinarifies { .. }
            | ByteArrayEffect::Encodes => {}
        }
    }

    /// Transfer function for `set name [expr …]`. Any binary/damaged use makes
    /// the result DAMAGED.
    fn track_assign_expr(
        &mut self,
        name: &str,
        span: Span,
        defs: &HashMap<Symbol, u32>,
        ssa: crate::ssa::SsaSourceView<'_>,
    ) {
        let nm = normalise_var_name(name);
        let Some(sym) = ssa.symbol(nm) else {
            return;
        };
        let Some(&ver) = defs.get(&sym) else {
            return;
        };
        let joined = join_prov(
            ssa.source_tokens()
                .into_iter()
                .flat_map(|tokens| {
                    tokens.variable_accesses.iter().filter_map(|access| {
                        let read = ssa.read_reference(&access.source, &access.original_spelling)?;
                        self.prov.get(&(read.symbol, read.version?)).cloned()
                    })
                })
                .map(Some),
        );
        if let Some(joined) = joined {
            self.prov.insert(
                (sym, ver),
                ByteProvInfo::damaged(&joined, Some(span), "expr".to_owned()),
            );
        }
    }

    /// Transfer function for a command call: a re-binarifier (`binary scan
    /// $v …`) restores its value operand's binary provenance in place; a
    /// read-modify-write coercer (`append v …`) damages its target variable;
    /// `<proto>::payload replace` is the byte sink. Each classification is
    /// registry data.
    fn track_call(
        &mut self,
        span: Span,
        defs: &HashMap<Symbol, u32>,
        uses: &HashMap<Symbol, u32>,
        ssa: crate::ssa::SsaSourceView<'_>,
    ) {
        let Some(invocation) = self.selected_invocation(ssa) else {
            return;
        };
        if let Some(assignment) = invocation.value_assignment() {
            self.track_stored_value(&assignment.name, &assignment.value, span, defs, ssa);
            return;
        }
        let ce = CmdEffect::selected(&invocation);
        let words = &invocation.effective_words().words;

        if self.track_encoder(span, &invocation, ssa) {
            return;
        }

        // A re-binarifier reads its value operand *as bytes*, re-installing
        // the byte-array rep in place (the documented fix) and clearing
        // DAMAGED. (`binary format … $v` is deliberately not stamped: which
        // args its cursors read as bytes depends on the format string, so a
        // damaged operand conservatively stays damaged.)
        if let ByteArrayEffect::Rebinarifies { value_arg } = ce.effect {
            if let Some(word) = words.get(ce.operand_start + usize::from(value_arg) + 1)
                && let Some(read) = ssa.read_word(word)
                && let Some(version) = read.version
                && let Some(old) = self.prov.get(&(read.symbol, version)).cloned()
            {
                self.prov.insert(
                    (read.symbol, version),
                    ByteProvInfo::binary(old.source_range, old.source_label),
                );
            }
            return;
        }

        // A read-modify-write string builder (`append v …`) coerces its
        // target variable to a character string when either the old target
        // value or an appended operand carries binary data. Selection is spec
        // data: a corrupting command-level effect on a command that
        // reads-then-writes the variable it assigns.
        if ce.effect.corrupts()
            && invocation.reads_before_write()
            && let Some((var_idx, _)) = invocation
                .operand_roles()
                .iter()
                .find(|(_, role)| *role == tcl_registry::ArgRole::VarWrite)
        {
            let var_idx = usize::from(*var_idx) + ce.operand_start;
            let Some(target) = invocation.argument_literal(var_idx) else {
                return;
            };
            let Some(target_sym) = ssa.symbol(normalise_var_name(&target)) else {
                return;
            };
            if let Some(&new_ver) = defs.get(&target_sym) {
                let old = self
                    .prov
                    .get(&(target_sym, uses.get(&target_sym).copied().unwrap_or(0)))
                    .cloned();
                let operand = join_prov(
                    std::iter::once(old).chain(
                        words[var_idx + 2..]
                            .iter()
                            .map(|word| self.arg_byte_prov(word, ssa, 0)),
                    ),
                );
                if let Some(op) = operand {
                    self.prov.insert(
                        (target_sym, new_ver),
                        ByteProvInfo::damaged(&op, Some(span), ce.label),
                    );
                }
            }
            return;
        }

        self.track_payload_sink(span, &invocation, ssa);
    }

    /// Encoding reads the data even when its result is discarded.
    fn track_encoder(
        &mut self,
        span: Span,
        invocation: &NormalRepresentationInvocation,
        source: crate::ssa::SsaSourceView<'_>,
    ) -> bool {
        if invocation.byte_array_effect() != ByteArrayEffect::Encodes {
            return false;
        }
        if let Some(operand) = self.operand_prov(invocation, source, 0) {
            self.warnings.push(byte_warning(
                span,
                "",
                &operand,
                format!(
                    "Byte-array corruption: '{}' on binary data from {} double-encodes it — \
                     the bytes are reinterpreted as characters and re-encoded. Decode with \
                     'encoding convertfrom' or keep it a byte array (S110)",
                    invocation.diagnostic_label(),
                    operand.source_label
                ),
            ));
        }
        true
    }

    /// Check the descriptor-selected payload value at its positioned read.
    fn track_payload_sink(
        &mut self,
        span: Span,
        invocation: &NormalRepresentationInvocation,
        ssa: crate::ssa::SsaSourceView<'_>,
    ) {
        let command = invocation.diagnostic_command();
        let words = &invocation.effective_words().words;
        if let Some(layout) = invocation.byte_array_payload()
            && let Some(data_idx) = payload_data_index(layout, invocation)
        {
            let Some(data_word) = words.get(data_idx + 1) else {
                return;
            };
            let info = self.arg_byte_prov(data_word, ssa, 0);
            if let Some(info) = info.filter(|i| i.state == ByteProv::Damaged) {
                let data_arg = data_word
                    .sole_variable_substitution()
                    .map_or("", |(spelling, _)| spelling)
                    .trim();
                let data_var = if data_word.sole_variable_substitution().is_some() {
                    normalise_var_name(data_arg).to_owned()
                } else {
                    String::new()
                };
                let coercion = if info.coercion_label.is_empty() {
                    "string operations"
                } else {
                    &info.coercion_label
                };
                let hint_var = if data_var.is_empty() {
                    "data"
                } else {
                    &data_var
                };
                let msg = format!(
                    "Byte-array corruption: '{command} {sink}' writes binary data from \
                     {label} that was modified as a character string ({coercion}); every byte \
                     >= 0x80 will be re-encoded. Re-binarify the value first, e.g. \
                     'binary scan ${hint_var} c* -' (S110)",
                    sink = BytePayloadSpec::REPLACE_SUB,
                    label = info.source_label,
                );
                self.warnings
                    .push(byte_warning(span, &data_var, &info, msg));
            }
        }
    }
}

/// Find byte-array-corruption (S110) warnings for a single function.
///
/// `payload_layouts` is the dialect-gated `*::payload` byte-command set (empty
/// under non-iRules dialects); the plain-Tcl `binary` / `encoding` sources are
/// always recognised via the registry.
#[must_use]
#[cfg(test)]
pub(crate) fn find_byte_array_warnings(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    executable_blocks: &HashSet<BlockId>,
    registry: &CommandRegistry,
    payload_layouts: &HashMap<&'static str, BytePayloadSpec>,
) -> Vec<ShimmerWarning> {
    find_byte_array_warnings_with_context(
        cfg,
        ssa,
        executable_blocks,
        super::ShimmerContext::standalone(registry),
        payload_layouts,
    )
}

pub(crate) fn find_byte_array_warnings_with_context(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    executable_blocks: &HashSet<BlockId>,
    context: super::ShimmerContext<'_>,
    payload_layouts: &HashMap<&'static str, BytePayloadSpec>,
) -> Vec<ShimmerWarning> {
    ByteCorruption::new(context, payload_layouts).run(cfg, ssa, executable_blocks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compilation_unit::CompilationUnit;
    use tcl_registry::CommandRegistry;

    fn irules_registry() -> std::sync::Arc<CommandRegistry> {
        tcl_registry::model::ingress::static_context_for("f5-irules")
            .commands()
            .clone()
    }

    /// Run the detector over `src`, returning the S110 warnings.
    fn warnings(src: &str, reg: &CommandRegistry) -> Vec<ShimmerWarning> {
        let cu = CompilationUnit::build_for(src, reg, false);
        let layouts = reg.byte_array_payload_layouts();
        let mut out = Vec::new();
        for fu in cu.analysable_functions() {
            out.extend(find_byte_array_warnings(
                &fu.cfg,
                &fu.ssa,
                &fu.sccp.executable_blocks,
                reg,
                &layouts,
            ));
        }
        out
    }

    fn assignment_proof_summary(src: &str, registry: &CommandRegistry) -> Vec<String> {
        let unit = CompilationUnit::build_for(src, registry, false);
        let layouts = registry.byte_array_payload_layouts();
        let tracker = ByteCorruption::new(super::ShimmerContext::standalone(registry), &layouts);
        let mut summary = Vec::new();
        for function in unit.analysable_functions() {
            for (&block, body) in &function.ssa.blocks {
                for (index, statement) in body.statements.iter().enumerate() {
                    let name = match &statement.statement {
                        Statement::AssignValue { name, .. } => name,
                        Statement::Call { command, .. } => command,
                        _ => continue,
                    };
                    let source =
                        crate::ssa::SsaSourceView::at_statement(&function.ssa, block, index);
                    let selected = tracker.selected_invocation(source);
                    let nested = selected
                        .as_ref()
                        .and_then(|call| call.effective_words().words.get(2))
                        .and_then(|word| tracker.nested_invocation(word, source));
                    let reads = source
                        .source_tokens()
                        .into_iter()
                        .flat_map(|tokens| &tokens.variable_accesses)
                        .map(|access| {
                            let read =
                                source.read_reference(&access.source, &access.original_spelling);
                            let contents = source.read_contents_at(
                                &access.source,
                                &access.original_spelling,
                                registry,
                            );
                            (access.original_spelling.clone(), read, contents)
                        })
                        .collect::<Vec<_>>();
                    summary.push(format!(
                        "{name}: defs {:?}; selected {:?}; nested {:?}; reads {reads:?}",
                        statement.defs,
                        selected.as_ref().map(|call| (
                            call.diagnostic_label(),
                            call.byte_array_effect(),
                            call.argument_offset()
                        )),
                        nested.as_ref().map(|call| (
                            call.diagnostic_label(),
                            call.returns_byte_array(),
                            call.byte_array_effect()
                        ))
                    ));
                }
            }
        }
        summary
    }

    /// A `TCP::payload` getter coerced via `string map` and written back
    /// through `TCP::payload replace` corrupts the bytes (S110).
    #[test]
    fn tcp_payload_string_replace_roundtrip_fires() {
        let reg = irules_registry();
        let src = "when CLIENT_DATA {\n  set p [TCP::payload]\n  set q [string map {a b} $p]\n  \
                   TCP::payload replace 0 100 $q\n}";
        let w = warnings(src, &reg);
        assert!(
            w.iter().any(|w| w.code == DiagCode::S110),
            "expected S110 for payload round-trip, got: {w:?}"
        );
    }

    /// `binary scan` re-binarifies the damaged value in place — the documented
    /// fix — so the subsequent `replace` is silent.
    #[test]
    fn binary_scan_rebinarify_fix_silent() {
        let reg = irules_registry();
        let src = "when CLIENT_DATA {\n  set p [TCP::payload]\n  set q [string map {a b} $p]\n  \
                   binary scan $q a* q\n  TCP::payload replace 0 100 $q\n}";
        let w = warnings(src, &reg);
        assert!(
            w.iter().all(|w| w.code != DiagCode::S110),
            "binary scan fix must silence S110, got: {w:?}"
        );
    }

    /// `binary encode` also reads its data operand as bytes, installing the
    /// byte-array rep in place (registry `Rebinarifies { value_arg: 1 }`,
    /// tclsh 8.6-verified), so it clears the damage like `binary scan`.
    #[test]
    fn binary_encode_rebinarify_silent() {
        let reg = irules_registry();
        let src = "when CLIENT_DATA {\n  set p [TCP::payload]\n  set q [string map {a b} $p]\n  \
                   binary encode hex $q\n  TCP::payload replace 0 100 $q\n}";
        let w = warnings(src, &reg);
        assert!(
            w.iter().all(|w| w.code != DiagCode::S110),
            "binary encode re-binarifies its data operand, got: {w:?}"
        );
    }

    /// A wrap-character option is read as text, separately from the binary
    /// input. Encoding another value cannot repair that option's provenance.
    #[test]
    fn binary_encode_does_not_clear_non_value_args() {
        let reg = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let src = "proc f {} {set p [binary format c 200]; set q [string map {a b} $p]; binary encode base64 -maxlen 1 -wrapchar $q extra; encoding convertto utf-8 $q}";
        let w = warnings(src, reg);
        assert!(
            w.iter().any(|w| w.code == DiagCode::S110),
            "damage outside the value_arg slot must survive, got: {w:?}; {:?}",
            assignment_proof_summary(src, reg)
        );
    }

    #[test]
    fn standalone_encoder_reads_only_its_data_operand() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let data = "proc f {} {set p [binary format c 200]; encoding convertto utf-8 $p}";
        assert!(
            warnings(data, registry)
                .iter()
                .any(|warning| warning.code == DiagCode::S110)
        );
        let name =
            "proc f {} {set codec [binary format a* utf-8]; encoding convertto $codec ordinary}";
        assert!(
            warnings(name, registry)
                .iter()
                .all(|warning| warning.code != DiagCode::S110)
        );
        let replaced = "rename encoding native_encoding; proc encoding args {return unchanged}; proc f {} {set p [binary format c 200]; encoding convertto utf-8 $p}";
        assert!(
            warnings(replaced, registry)
                .iter()
                .all(|warning| warning.code != DiagCode::S110)
        );
    }

    /// A clean writeback (getter → replace, no string coercion) is safe.
    #[test]
    fn clean_payload_writeback_silent() {
        let reg = irules_registry();
        let src = "when CLIENT_DATA {\n  set p [TCP::payload]\n  TCP::payload replace 0 100 $p\n}";
        let w = warnings(src, &reg);
        assert_eq!(w.len(), 0, "clean writeback must be silent, got: {w:?}");
    }

    /// Plain-Tcl `string toupper` on a `binary format` value fires immediately
    /// (case folding corrupts directly, no sink needed).
    #[test]
    fn plain_tcl_toupper_case_fold_fires() {
        let reg = CommandRegistry::build_default().project_for_profile(
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        );
        let src = "proc f {} {\n  set b [binary format a* hello]\n  set u [string toupper $b]\n}";
        let w = warnings(src, &reg);
        assert!(
            w.iter().any(|w| w.code == DiagCode::S110),
            "expected S110 for case fold on binary, got: {w:?}; {:?}",
            assignment_proof_summary(src, &reg)
        );
    }

    /// `encoding convertto` on an already-binary value double-encodes (S110).
    #[test]
    fn encoding_convertto_double_encode_fires() {
        let reg = CommandRegistry::build_default().project_for_profile(
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        );
        let src = "proc f {} {\n  set b [binary format a* hello]\n  set e [encoding convertto utf-8 $b]\n}";
        let w = warnings(src, &reg);
        assert!(
            w.iter().any(|w| w.code == DiagCode::S110),
            "expected S110 for convertto double-encode, got: {w:?}"
        );
        let replaced = format!("proc ::tcl::encoding::convertto {{args}} {{return safe}}\n{src}");
        assert!(
            warnings(&replaced, &reg)
                .iter()
                .all(|warning| warning.code != DiagCode::S110),
            "a replaced private conversion worker cannot donate the stock encoding effect"
        );
    }

    /// A plain string passed through `string toupper` is untracked — no S110.
    #[test]
    fn toupper_plain_string_silent() {
        let reg = CommandRegistry::build_default();
        let src = "proc f {} {\n  set s \"hello\"\n  set u [string toupper $s]\n}";
        let w = warnings(src, &reg);
        assert_eq!(w.len(), 0, "plain string toupper must be silent: {w:?}");
    }

    /// A document that merely names `*::payload` under a non-iRules dialect must
    /// not trip — the payload layout set is empty without the iRules pack.
    #[test]
    fn payload_names_outside_irules_dialect_silent() {
        let reg = CommandRegistry::build_default();
        let src = "proc f {} {\n  set p [TCP::payload]\n  set q [string map {a b} $p]\n  \
                   TCP::payload replace 0 100 $q\n}";
        let w = warnings(src, &reg);
        assert_eq!(
            w.len(),
            0,
            "payload names under plain Tcl must be silent: {w:?}"
        );
    }

    /// A pure-copy chain preserves provenance: the damage flows through `set r
    /// $q` to the sink.
    #[test]
    fn copy_chain_preserves_provenance_fires() {
        let reg = irules_registry();
        let src = "when CLIENT_DATA {\n  set p [TCP::payload]\n  set q [string map {a b} $p]\n  \
                   set r $q\n  TCP::payload replace 0 100 $r\n}";
        let w = warnings(src, &reg);
        assert!(
            w.iter().any(|w| w.code == DiagCode::S110),
            "expected S110 through copy chain, got: {w:?}"
        );
    }

    #[test]
    fn inert_word_text_never_creates_a_binary_source_or_read() {
        let reg = irules_registry();
        for value in ["{[TCP::payload]}", "{$binary}"] {
            let source = format!(
                "when CLIENT_DATA {{
set binary [TCP::payload]
set p {value}
\
                set q [string map {{a b}} $p]
TCP::payload replace 0 1 $q
}}"
            );
            assert!(
                !fires_s110(&source, &reg),
                "inert value {value}: {:?}",
                warnings(&source, &reg)
            );
        }
    }

    #[test]
    fn replacement_binary_command_does_not_inherit_a_source_contract() {
        let registry = CommandRegistry::build_default();
        let source = "rename binary saved_binary
proc binary {args} {return TEXT}
\
            set p [binary format c 200]
set q [string tolower $p]";
        assert!(
            !fires_s110(source, &registry),
            "{:?}",
            warnings(source, &registry)
        );
    }

    /// Empty source: no warnings, no panic.
    #[test]
    fn empty_source_silent() {
        let reg = irules_registry();
        let w = warnings("", &reg);
        assert_eq!(w, [] as [crate::shimmer::ShimmerWarning; 0]);
    }

    fn fires_s110(src: &str, reg: &CommandRegistry) -> bool {
        warnings(src, reg).iter().any(|w| w.code == DiagCode::S110)
    }

    /// FP fix: `string range`/`index`/`reverse` keep the byte-array
    /// representation in both tclsh 8.6 and 9.0, so a `payload` getter passed
    /// through one of them and written back is byte-exact and must NOT fire
    /// S110 — the canonical `string range $payload …` → `payload replace`
    /// idiom.
    #[test]
    fn transparent_string_ops_on_payload_are_silent() {
        let reg = irules_registry();
        // Two-statement form.
        for op in [
            "string range $p 0 5",
            "string index $p 3",
            "string reverse $p",
        ] {
            let src = format!(
                "when CLIENT_DATA {{\n  set p [TCP::payload]\n  set q [{op}]\n  \
                 TCP::payload replace 0 100 $q\n}}"
            );
            assert!(
                !fires_s110(&src, &reg),
                "transparent op '{op}' must not fire S110 (two-statement), got: {:?}",
                warnings(&src, &reg),
            );
            // Inline sink form.
            let inline = format!(
                "when CLIENT_DATA {{\n  set p [TCP::payload]\n  \
                 TCP::payload replace 0 100 [{op}]\n}}"
            );
            assert!(
                !fires_s110(&inline, &reg),
                "transparent op '{op}' must not fire S110 (inline), got: {:?}",
                warnings(&inline, &reg),
            );
        }
    }

    /// TP control: the coercing `string` value builders still corrupt a byte
    /// array (they produce a character string), so they still fire.
    #[test]
    fn coercing_string_ops_on_payload_still_fire() {
        let reg = irules_registry();
        for op in [
            "string map {a b} $p",
            "string replace $p 0 0 Z",
            "string repeat $p 2",
        ] {
            let src = format!(
                "when CLIENT_DATA {{\n  set p [TCP::payload]\n  set q [{op}]\n  \
                 TCP::payload replace 0 100 $q\n}}"
            );
            assert!(
                fires_s110(&src, &reg),
                "coercing op '{op}' must fire S110, got: {:?}",
                warnings(&src, &reg),
            );
        }
    }

    #[test]
    fn string_insert_damage_uses_the_release_where_the_handler_exists() {
        let reg = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        let source = "proc f {} {set p [binary format c 200]; set q [string insert $p 0 Z]; set e [encoding convertto utf-8 $q]}";
        assert!(fires_s110(source, reg), "{:?}", warnings(source, reg));
        let reg = irules_registry();
        let unavailable = "when CLIENT_DATA {set p [TCP::payload]; set q [string insert $p 0 Z]; TCP::payload replace 0 100 $q}";
        assert!(!fires_s110(unavailable, &reg));
    }

    /// Fire half: `string trim`/`trimleft`/`trimright` build a fresh
    /// character string whenever they actually trim, in both tclsh 8.6 and 9.0
    /// (`StringTrimCmd` → `Tcl_NewStringObj`; the compiled `INST_STR_TRIM`
    /// keeps the object only for a no-op trim). They are not
    /// byte-array-transparent, so a trimmed payload written back must fire.
    #[test]
    fn trim_ops_on_payload_fire() {
        let reg = irules_registry();
        for op in [
            "string trim $p",
            "string trimleft $p",
            "string trimright $p",
        ] {
            let src = format!(
                "when CLIENT_DATA {{\n  set p [TCP::payload]\n  set q [{op}]\n  \
                 TCP::payload replace 0 100 $q\n}}"
            );
            assert!(
                fires_s110(&src, &reg),
                "coercing op '{op}' must fire S110, got: {:?}",
                warnings(&src, &reg),
            );
        }
    }

    /// 9.0-source-corrected classification (silent half): trimming an
    /// untracked plain string is not byte-array corruption.
    #[test]
    fn trim_plain_string_silent() {
        let reg = irules_registry();
        let src = "when CLIENT_DATA {\n  set s \"hello \"\n  set q [string trim $s]\n  \
                   TCP::payload replace 0 100 $q\n}";
        assert!(
            !fires_s110(src, &reg),
            "trim on an untracked string must be silent, got: {:?}",
            warnings(src, &reg),
        );
    }

    /// A transparent op followed by a coercing op still fires: the transparent
    /// op propagates the binary provenance, then `string map` damages it.
    #[test]
    fn transparent_then_coerced_still_fires() {
        let reg = irules_registry();
        let src = "when CLIENT_DATA {\n  set p [TCP::payload]\n  set q [string range $p 0 5]\n  \
                   set r [string map {a b} $q]\n  TCP::payload replace 0 100 $r\n}";
        assert!(
            fires_s110(src, &reg),
            "range→map→replace must fire S110, got: {:?}",
            warnings(src, &reg),
        );
    }

    /// A transparent op preserves an *already-damaged* value's state: a value
    /// coerced by `string map` and then passed through `string range` is still
    /// damaged at the sink.
    #[test]
    fn transparent_preserves_prior_damage() {
        let reg = irules_registry();
        let src = "when CLIENT_DATA {\n  set p [TCP::payload]\n  set d [string map {a b} $p]\n  \
                   set q [string range $d 0 5]\n  TCP::payload replace 0 100 $q\n}";
        assert!(
            fires_s110(src, &reg),
            "map→range→replace must still fire S110 (damage survives a transparent op), got: {:?}",
            warnings(src, &reg),
        );
    }

    /// `append` damages its target variable (registry: command-level
    /// `Coerces` + `READS_BEFORE_WRITE` + `assigns_variable_at 0` — no
    /// hardcoded name).
    #[test]
    fn append_damages_target_fires() {
        let reg = irules_registry();
        let src = "when CLIENT_DATA {\n  set p [TCP::payload]\n  append p \" tail\"\n  \
                   TCP::payload replace 0 100 $p\n}";
        assert!(
            fires_s110(src, &reg),
            "append must damage its target, got: {:?}",
            warnings(src, &reg),
        );
    }

    /// Drift guard: the registry-resolved classification reproduces the
    /// source / re-binarify / label behaviour pinned here for the known
    /// commands.
    #[test]
    fn registry_resolution_matches_legacy_hardcoded_sets() {
        let reg = irules_registry();
        let args = |a: &[&str]| a.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();

        // Binary sources: exactly `binary format` / `binary decode` /
        // `encoding convertto` (plus payload getters below).
        for (cmd, sub) in [("binary", "format"), ("binary", "decode")] {
            let ce = resolve_cmd_effect(&reg, cmd, &args(&[sub, "x", "y"]));
            assert!(ce.returns_byte_array, "{cmd} {sub} must be a source");
            assert_eq!(ce.label, format!("{cmd} {sub}"));
            assert_eq!(ce.operand_start, 1);
        }
        let convertto = resolve_cmd_effect(&reg, "encoding", &args(&["convertto", "utf-8", "$b"]));
        assert!(convertto.returns_byte_array);
        assert_eq!(convertto.effect, ByteArrayEffect::Encodes);
        assert_eq!(convertto.label, "encoding convertto");

        // Non-sources stay non-sources.
        for (cmd, rest) in [
            ("binary", &["scan", "$b", "c*", "v"][..]),
            ("binary", &["encode", "hex", "$b"][..]),
            ("encoding", &["convertfrom", "utf-8", "$b"][..]),
            ("string", &["map", "{a b}", "$b"][..]),
            ("format", &["%s", "$b"][..]),
        ] {
            let ce = resolve_cmd_effect(&reg, cmd, &args(rest));
            assert!(
                !ce.returns_byte_array,
                "{cmd} {} must not be a source",
                rest[0]
            );
        }

        // Re-binarifiers: exactly `binary scan` (value at sub-relative 0,
        // i.e. call arg 1) and `binary encode` (value at sub-relative 1).
        let scan = resolve_cmd_effect(&reg, "binary", &args(&["scan", "$q", "a*", "q"]));
        assert_eq!(scan.effect, ByteArrayEffect::Rebinarifies { value_arg: 0 });
        // `operand_start + value_arg` = call arg 1.
        assert_eq!(scan.operand_start, 1);
        let encode = resolve_cmd_effect(&reg, "binary", &args(&["encode", "hex", "$q"]));
        assert_eq!(
            encode.effect,
            ByteArrayEffect::Rebinarifies { value_arg: 1 }
        );

        // Payload getter/non-getter split for the known payload commands.
        let layouts = reg.byte_array_payload_layouts();
        for proto in [
            "TCP::payload",
            "UDP::payload",
            "HTTP::payload",
            "SCTP::payload",
            "GTP::payload",
            "MQTT::payload",
            "DIAMETER::payload",
        ] {
            assert!(layouts.contains_key(proto), "{proto} layout missing");
            assert!(is_payload_getter(&layouts, proto, &args(&[])), "{proto}");
            assert!(
                is_payload_getter(&layouts, proto, &args(&["100"])),
                "{proto} 100"
            );
            for sub in ["replace", "length", "rechunk", "unchunk"] {
                assert!(
                    !is_payload_getter(&layouts, proto, &args(&[sub, "x"])),
                    "{proto} {sub} must not be a getter",
                );
            }
        }
        // Sink data slots, per layout.
        assert_eq!(
            layouts["TCP::payload"].replace_data_arg(&["replace", "0", "100", "$q"]),
            Some(3),
        );
        assert_eq!(
            layouts["MQTT::payload"].replace_data_arg(&["replace", "$bad"]),
            Some(1),
        );
        assert_eq!(
            layouts["GTP::payload"]
                .replace_data_arg(&["replace", "-message", "$m", "0", "100", "$q"]),
            Some(5),
        );
    }
}
