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
//! See `docs/design/rust/s110-byte-array-corruption-port.md`.
//!
//! [`CommandSpec::byte_array_payload`]: tcl_registry::CommandSpec::byte_array_payload

use std::collections::HashMap;
use tcl_core_types::DiagCode;

use tcl_lexer::Span;
use tcl_registry::{ByteArrayEffect, BytePayloadSpec, CommandRegistry, TclType, Traits};

use crate::cfg::{BlockId, Function as CfgFunction};
use crate::ir::Statement;
use crate::naming::normalise_var_name;
use crate::sccp::cfg_order;
use crate::ssa::{SsaFunction, Symbol, ValueKey};
use crate::value_shapes::{is_pure_var_ref, parse_command_substitution};
use crate::var_refs::vars_in_word;

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
/// This replaces the command/subcommand name lists the pass used to hardcode:
/// the effect, the byte-array-source flag, the operand window, and the
/// diagnostic label are all spec data.
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
    /// An inert classification labelled with the bare command word.
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
fn resolve_cmd_effect(registry: &CommandRegistry, cmd: &str, args: &[String]) -> CmdEffect {
    let Some(spec) = registry.get(cmd) else {
        return CmdEffect::inert(cmd);
    };
    if spec.subcommands.is_empty() {
        return CmdEffect {
            effect: spec.byte_array_effect,
            returns_byte_array: spec.return_type == Some(TclType::ByteArray),
            operand_start: 0,
            label: cmd.to_owned(),
        };
    }
    match args.first().and_then(|w| spec.resolve_subcommand(w)) {
        Some(sub) => CmdEffect {
            effect: sub.byte_array_effect,
            returns_byte_array: sub.return_type == Some(TclType::ByteArray),
            operand_start: 1,
            label: format!("{cmd} {}", sub.name),
        },
        None => CmdEffect::inert(cmd),
    }
}

/// True when `[cmd args]` reads raw payload bytes (the getter form of a
/// registry `*::payload` byte command).
fn is_payload_getter(
    layouts: &HashMap<&'static str, BytePayloadSpec>,
    cmd: &str,
    args: &[String],
) -> bool {
    layouts.contains_key(cmd) && BytePayloadSpec::is_getter_call(args.first().map(String::as_str))
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
    registry: &'a CommandRegistry,
    payload_layouts: &'a HashMap<&'static str, BytePayloadSpec>,
    prov: HashMap<ValueKey, ByteProvInfo>,
    warnings: Vec<ShimmerWarning>,
}

impl<'a> ByteCorruption<'a> {
    fn new(
        registry: &'a CommandRegistry,
        payload_layouts: &'a HashMap<&'static str, BytePayloadSpec>,
    ) -> Self {
        Self {
            registry,
            payload_layouts,
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

            for ss in &ssa_block.statements {
                match &ss.statement {
                    Statement::AssignValue {
                        name, value, span, ..
                    } => self.track_assign_value(name, value, *span, &ss.defs, &ss.uses, ssa),
                    Statement::AssignExpr { name, span, .. } => {
                        self.track_assign_expr(name, *span, &ss.defs, &ss.uses, ssa);
                    }
                    Statement::Call {
                        command,
                        args,
                        span,
                        ..
                    } => self.track_call(command, args, *span, &ss.defs, &ss.uses, ssa),
                    _ => {}
                }
            }
        }
        self.warnings
    }

    /// Normalised variable names referenced via `$var` / `${var}` in `text`.
    fn vars_in(&self, text: &str) -> Vec<String> {
        vars_in_word(text, self.registry).into_iter().collect()
    }

    /// Joined provenance of the value operands of `[cmd args…]` (the args
    /// after the resolved subcommand word, when there is one).
    fn operand_prov(
        &self,
        ce: &CmdEffect,
        cargs: &[String],
        uses: &HashMap<Symbol, u32>,
        ssa: &SsaFunction,
    ) -> Option<ByteProvInfo> {
        join_prov(
            cargs[ce.operand_start.min(cargs.len())..]
                .iter()
                .map(|c| self.arg_byte_prov(c, uses, ssa)),
        )
    }

    /// Provenance of a single argument expression (var ref, command
    /// substitution, or interpolation).
    fn arg_byte_prov(
        &self,
        arg_text: &str,
        uses: &HashMap<Symbol, u32>,
        ssa: &SsaFunction,
    ) -> Option<ByteProvInfo> {
        let a = arg_text.trim();
        if a.is_empty() {
            return None;
        }
        if is_pure_var_ref(a) {
            let name = normalise_var_name(a);
            let sym = ssa.var_symbol(name)?;
            let ver = uses.get(&sym).copied().unwrap_or(0);
            return (ver > 0)
                .then(|| self.prov.get(&(sym, ver)).cloned())
                .flatten();
        }
        if let Some((cmd, cargs)) = parse_command_substitution(a) {
            let ce = resolve_cmd_effect(self.registry, &cmd, &cargs);
            if is_payload_getter(self.payload_layouts, &cmd, &cargs) {
                return Some(ByteProvInfo::binary(None, cmd));
            }
            // An encoder is *not* the clean source its byte-array return type
            // suggests when its operand is already binary — that is the
            // double-encode case, handled in the effect dispatch below so an
            // inline sink form `<proto>::payload replace … [encoding
            // convertto …]` is damaged, not clean.
            if ce.effect != ByteArrayEffect::Encodes && ce.returns_byte_array {
                return Some(ByteProvInfo::binary(None, ce.label));
            }
            // A transform applied to a (possibly binary) operand inside the
            // arg — the registry effect decides whether the byte-array
            // survives.
            let operand = self.operand_prov(&ce, &cargs, uses, ssa);
            return match ce.effect {
                // Encoding a binary operand double-encodes it (DAMAGED); on
                // non-binary data the result is a legitimate byte source.
                ByteArrayEffect::Encodes => Some(match operand {
                    Some(op) => ByteProvInfo::damaged(&op, None, ce.label),
                    None => ByteProvInfo::binary(None, ce.label),
                }),
                // Result keeps the byte-array rep — propagate provenance as-is.
                ByteArrayEffect::Transparent => operand,
                // Case-fold / coerce turn a binary operand into a damaged
                // string. An unmodelled transform (`None`) or a nested
                // re-binarifier (whose in-place effect applies only as a
                // command statement) conservatively damages an inner binary
                // operand too (the prior default).
                ByteArrayEffect::CaseFolds
                | ByteArrayEffect::Coerces
                | ByteArrayEffect::None
                | ByteArrayEffect::Rebinarifies { .. } => {
                    operand.map(|inner| ByteProvInfo::damaged(&inner, None, ce.label))
                }
            };
        }
        if a.contains('$') || a.contains('[') {
            let joined = join_prov(self.interpolated_prov(a, uses, ssa));
            if let Some(joined) = joined {
                return Some(ByteProvInfo::damaged(
                    &joined,
                    None,
                    "interpolation".to_owned(),
                ));
            }
        }
        None
    }

    /// Provenance of each `$var` referenced inside an interpolated word.
    fn interpolated_prov(
        &self,
        text: &str,
        uses: &HashMap<Symbol, u32>,
        ssa: &SsaFunction,
    ) -> Vec<Option<ByteProvInfo>> {
        self.vars_in(text)
            .into_iter()
            .filter_map(|name| {
                let sym = ssa.var_symbol(&name)?;
                let ver = uses.get(&sym).copied().unwrap_or(0);
                (ver > 0).then(|| self.prov.get(&(sym, ver)).cloned())
            })
            .collect()
    }

    /// Transfer function for `set name value`.
    fn track_assign_value(
        &mut self,
        name: &str,
        value: &str,
        span: Span,
        defs: &HashMap<Symbol, u32>,
        uses: &HashMap<Symbol, u32>,
        ssa: &SsaFunction,
    ) {
        let nm = normalise_var_name(name).to_owned();
        let Some(sym) = ssa.var_symbol(&nm) else {
            return;
        };
        let Some(&ver) = defs.get(&sym) else {
            return;
        };
        let key = (sym, ver);
        let val = value.trim();

        if let Some((cmd, cargs)) = parse_command_substitution(val) {
            if is_payload_getter(self.payload_layouts, &cmd, &cargs) {
                self.prov.insert(key, ByteProvInfo::binary(Some(span), cmd));
                return;
            }
            let ce = resolve_cmd_effect(self.registry, &cmd, &cargs);
            // An encoder on an already-binary value is the double-encode bug —
            // warn, then treat the (byte-array) result as binary.
            if ce.effect == ByteArrayEffect::Encodes {
                let operand = self.operand_prov(&ce, &cargs, uses, ssa);
                if let Some(op) = operand {
                    let msg = format!(
                        "Byte-array corruption: '{label}' on binary data from \
                         {src} double-encodes it — the bytes are reinterpreted as characters \
                         and re-encoded. Decode with 'encoding convertfrom' or keep it a byte \
                         array (S110)",
                        label = ce.label,
                        src = op.source_label,
                    );
                    self.warnings.push(byte_warning(span, &nm, &op, msg));
                }
                self.prov
                    .insert(key, ByteProvInfo::binary(Some(span), ce.label));
                return;
            }
            if ce.returns_byte_array {
                self.prov
                    .insert(key, ByteProvInfo::binary(Some(span), ce.label));
                return;
            }
            // Registry-declared byte-array effect of this transform (the
            // resolved subcommand's effect, or a whole command's) — never a
            // hardcoded command name.
            let operand = self.operand_prov(&ce, &cargs, uses, ssa);
            self.apply_byte_array_effect(&ce, key, span, &nm, operand);
            return;
        }

        if is_pure_var_ref(val) {
            let src = normalise_var_name(val);
            if let Some(src_sym) = ssa.var_symbol(src) {
                let src_ver = uses.get(&src_sym).copied().unwrap_or(0);
                if src_ver > 0
                    && let Some(info) = self.prov.get(&(src_sym, src_ver)).cloned()
                {
                    self.prov.insert(key, info);
                }
            }
            return;
        }

        if val.contains('$') || val.contains('[') {
            let joined = join_prov(self.interpolated_prov(val, uses, ssa));
            if let Some(joined) = joined {
                self.prov.insert(
                    key,
                    ByteProvInfo::damaged(&joined, Some(span), "string interpolation".to_owned()),
                );
            }
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
        uses: &HashMap<Symbol, u32>,
        ssa: &SsaFunction,
    ) {
        let nm = normalise_var_name(name);
        let Some(sym) = ssa.var_symbol(nm) else {
            return;
        };
        let Some(&ver) = defs.get(&sym) else {
            return;
        };
        let joined = join_prov(
            uses.iter()
                .filter(|(_, v)| **v > 0)
                .map(|(&n, v)| self.prov.get(&(n, *v)).cloned())
                .collect::<Vec<_>>(),
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
        command: &str,
        args: &[String],
        span: Span,
        defs: &HashMap<Symbol, u32>,
        uses: &HashMap<Symbol, u32>,
        ssa: &SsaFunction,
    ) {
        let ce = resolve_cmd_effect(self.registry, command, args);

        // A re-binarifier reads its value operand *as bytes*, re-installing
        // the byte-array rep in place (the documented fix) and clearing
        // DAMAGED. (`binary format … $v` is deliberately not stamped: which
        // args its cursors read as bytes depends on the format string, so a
        // damaged operand conservatively stays damaged.)
        if let ByteArrayEffect::Rebinarifies { value_arg } = ce.effect {
            if let Some(arg) = args.get(ce.operand_start + usize::from(value_arg)) {
                let a = arg.trim();
                if is_pure_var_ref(a)
                    && let Some(sym) = ssa.var_symbol(normalise_var_name(a))
                {
                    let v = uses.get(&sym).copied().unwrap_or(0);
                    if v > 0
                        && let Some(old) = self.prov.get(&(sym, v)).cloned()
                    {
                        self.prov.insert(
                            (sym, v),
                            ByteProvInfo::binary(old.source_range, old.source_label),
                        );
                    }
                }
            }
            return;
        }

        // A read-modify-write string builder (`append v …`) coerces its
        // target variable to a character string when either the old target
        // value or an appended operand carries binary data. Selection is spec
        // data: a corrupting command-level effect on a command that
        // reads-then-writes the variable it assigns.
        if ce.effect.corrupts()
            && let Some(spec) = self.registry.get(command)
            && spec.traits.contains(Traits::READS_BEFORE_WRITE)
            && let Some(var_idx) = spec.assigns_variable_at
        {
            let var_idx = usize::from(var_idx);
            let Some(target) = args.get(var_idx) else {
                return;
            };
            let Some(target_sym) = ssa.var_symbol(normalise_var_name(target)) else {
                return;
            };
            if let Some(&new_ver) = defs.get(&target_sym) {
                let old = self
                    .prov
                    .get(&(target_sym, uses.get(&target_sym).copied().unwrap_or(0)))
                    .cloned();
                let operand = join_prov(
                    std::iter::once(old).chain(
                        args[var_idx + 1..]
                            .iter()
                            .map(|a| self.arg_byte_prov(a, uses, ssa)),
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

        // Sink: `<proto>::payload replace … <data>` (the data index is
        // per-protocol registry layout data).
        if let Some(layout) = self.payload_layouts.get(command)
            && let Some(data_idx) = layout.replace_data_arg(args)
        {
            let info = self.arg_byte_prov(&args[data_idx], uses, ssa);
            if let Some(info) = info.filter(|i| i.state == ByteProv::Damaged) {
                let data_arg = args[data_idx].trim();
                let data_var = if is_pure_var_ref(data_arg) {
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
pub(crate) fn find_byte_array_warnings(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    executable_blocks: &HashSet<BlockId>,
    registry: &CommandRegistry,
    payload_layouts: &HashMap<&'static str, BytePayloadSpec>,
) -> Vec<ShimmerWarning> {
    ByteCorruption::new(registry, payload_layouts).run(cfg, ssa, executable_blocks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compilation_unit::CompilationUnit;
    use tcl_registry::CommandRegistry;

    fn irules_registry() -> CommandRegistry {
        let mut reg = CommandRegistry::build_default();
        reg.load_irules();
        reg
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

    /// Control for the re-binarifier arg index: `binary encode`'s *format*
    /// word is not the value operand, so a damaged variable used elsewhere in
    /// the call is not cleared.
    #[test]
    fn binary_encode_does_not_clear_non_value_args() {
        let reg = irules_registry();
        let src = "when CLIENT_DATA {\n  set p [TCP::payload]\n  set q [string map {a b} $p]\n  \
                   binary encode hex extra $q\n  TCP::payload replace 0 100 $q\n}";
        // `binary encode hex extra $q` puts `$q` outside the modelled
        // `value_arg` slot (sub-relative 1 is `extra`), so the damage must
        // survive to the sink.
        let w = warnings(src, &reg);
        assert!(
            w.iter().any(|w| w.code == DiagCode::S110),
            "damage outside the value_arg slot must survive, got: {w:?}"
        );
    }

    /// A clean writeback (getter → replace, no string coercion) is safe.
    #[test]
    fn clean_payload_writeback_silent() {
        let reg = irules_registry();
        let src = "when CLIENT_DATA {\n  set p [TCP::payload]\n  TCP::payload replace 0 100 $p\n}";
        let w = warnings(src, &reg);
        assert!(w.is_empty(), "clean writeback must be silent, got: {w:?}");
    }

    /// Plain-Tcl `string toupper` on a `binary format` value fires immediately
    /// (case folding corrupts directly, no sink needed).
    #[test]
    fn plain_tcl_toupper_case_fold_fires() {
        let reg = CommandRegistry::build_default();
        let src = "proc f {} {\n  set b [binary format a* hello]\n  set u [string toupper $b]\n}";
        let w = warnings(src, &reg);
        assert!(
            w.iter().any(|w| w.code == DiagCode::S110),
            "expected S110 for case fold on binary, got: {w:?}"
        );
    }

    /// `encoding convertto` on an already-binary value double-encodes (S110).
    #[test]
    fn encoding_convertto_double_encode_fires() {
        let reg = CommandRegistry::build_default();
        let src = "proc f {} {\n  set b [binary format a* hello]\n  set e [encoding convertto utf-8 $b]\n}";
        let w = warnings(src, &reg);
        assert!(
            w.iter().any(|w| w.code == DiagCode::S110),
            "expected S110 for convertto double-encode, got: {w:?}"
        );
    }

    /// A plain string passed through `string toupper` is untracked — no S110.
    #[test]
    fn toupper_plain_string_silent() {
        let reg = CommandRegistry::build_default();
        let src = "proc f {} {\n  set s \"hello\"\n  set u [string toupper $s]\n}";
        let w = warnings(src, &reg);
        assert!(w.is_empty(), "plain string toupper must be silent: {w:?}");
    }

    /// A document that merely names `*::payload` under a non-iRules dialect must
    /// not trip — the payload layout set is empty without the iRules pack.
    #[test]
    fn payload_names_outside_irules_dialect_silent() {
        let reg = CommandRegistry::build_default();
        let src = "proc f {} {\n  set p [TCP::payload]\n  set q [string map {a b} $p]\n  \
                   TCP::payload replace 0 100 $q\n}";
        let w = warnings(src, &reg);
        assert!(
            w.is_empty(),
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

    /// Empty source: no warnings, no panic.
    #[test]
    fn empty_source_silent() {
        let reg = irules_registry();
        let w = warnings("", &reg);
        assert!(w.is_empty());
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
            "string insert $p 0 Z",
            "string cat $p $p",
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

    /// 9.0-source-corrected classification (fire half): `string trim`/
    /// `trimleft`/`trimright` build a fresh character string whenever they
    /// actually trim, in both tclsh 8.6 and 9.0 (`StringTrimCmd` →
    /// `Tcl_NewStringObj`; the compiled `INST_STR_TRIM` keeps the object only
    /// for a no-op trim). They were previously misclassified as
    /// byte-array-transparent, so a trimmed payload written back must now
    /// fire.
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
    /// legacy hardcoded source / re-binarify / label behaviour for the known
    /// commands the pass used to name.
    #[test]
    fn registry_resolution_matches_legacy_hardcoded_sets() {
        let reg = irules_registry();
        let args = |a: &[&str]| a.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();

        // Binary sources: exactly the legacy `binary format` / `binary
        // decode` / `encoding convertto` (plus payload getters below).
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
        // i.e. call arg 1 — the legacy hardcoded slot) and `binary encode`
        // (value at sub-relative 1).
        let scan = resolve_cmd_effect(&reg, "binary", &args(&["scan", "$q", "a*", "q"]));
        assert_eq!(scan.effect, ByteArrayEffect::Rebinarifies { value_arg: 0 });
        // `operand_start + value_arg` = call arg 1, the legacy hardcoded slot.
        assert_eq!(scan.operand_start, 1);
        let encode = resolve_cmd_effect(&reg, "binary", &args(&["encode", "hex", "$q"]));
        assert_eq!(
            encode.effect,
            ByteArrayEffect::Rebinarifies { value_arg: 1 }
        );

        // Payload getter/non-getter split matches the legacy
        // `PAYLOAD_NON_GETTER_SUBS` list for the known payload commands.
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
        // Sink data slots: the legacy hardcoded index logic per layout.
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
