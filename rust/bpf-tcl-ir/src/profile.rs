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

//! The profile-based top layer (protocol facet).
//!
//! A **profile** is a named, reusable bundle the program selects (mirroring F5's
//! `ProfileSpec`). This module implements the *protocol* facet: a profile
//! declares named header **fields** `(offset, width)`, and a handler reads a
//! field by name — `tcp_dport dport` — which the front-end expands (before CFG
//! construction) into `load<width> dport __pkt <offset>`, reusing `lower_load`.
//!
//! Profiles are **built-in** (a shipped library: `eth`/`ipv4`/`tcp`/`udp` and
//! the combined `ipv4_tcp`/`ipv4_udp`) or **user-defined** in the source
//! (`profile NAME { field … }`). Offsets assume an Ethernet(14) + IPv4(20,
//! no options) + L4 layout. Other facets (templates, capability, deployment)
//! layer onto this same `BpfProfileSpec` in later increments.

use tcl_compiler::ir::IfClause;
use tcl_compiler::lowering::lower_to_ir;
use tcl_compiler::{Script, Statement};
use tcl_lexer::Span;
use tcl_registry::registry::CommandRegistry;

use crate::diag::{BpfDiag, BpfError};

/// A named header field at a fixed packet offset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDef {
    /// The field-access command name (e.g. `tcp_dport`).
    pub name: String,
    /// Byte offset from the packet start.
    pub offset: i32,
    /// Width in bits (8, 16, or 32).
    pub width_bits: u8,
}

/// A profile: a named bundle of header fields (+ later facets).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BpfProfileSpec {
    /// Profile name.
    pub name: String,
    /// Header fields the profile exposes as commands.
    pub fields: Vec<FieldDef>,
}

impl BpfProfileSpec {
    /// Look up a field by its command name.
    #[must_use]
    pub fn field(&self, name: &str) -> Option<&FieldDef> {
        self.fields.iter().find(|f| f.name == name)
    }
}

fn field(name: &str, offset: i32, width_bits: u8) -> FieldDef {
    FieldDef {
        name: name.to_owned(),
        offset,
        width_bits,
    }
}

/// Resolve a built-in profile by name (case-insensitive).
#[must_use]
pub fn lookup_builtin(name: &str) -> Option<BpfProfileSpec> {
    // Standard Ethernet(14)+IPv4(20)+L4 offsets.
    let eth = [field("eth_type", 12, 16)];
    let ipv4 = [
        field("ip_ttl", 22, 8),
        field("ip_proto", 23, 8),
        field("ip_saddr", 26, 32),
        field("ip_daddr", 30, 32),
    ];
    let tcp = [
        field("tcp_sport", 34, 16),
        field("tcp_dport", 36, 16),
        field("tcp_flags", 47, 8),
    ];
    let udp = [
        field("udp_sport", 34, 16),
        field("udp_dport", 36, 16),
        field("udp_len", 38, 16),
    ];
    let make = |name: &str, groups: &[&[FieldDef]]| BpfProfileSpec {
        name: name.to_owned(),
        fields: groups.iter().flat_map(|g| g.iter().cloned()).collect(),
    };
    match name.to_ascii_lowercase().as_str() {
        "eth" => Some(make("eth", &[&eth])),
        "ipv4" => Some(make("ipv4", &[&eth, &ipv4])),
        "tcp" => Some(make("tcp", &[&tcp])),
        "udp" => Some(make("udp", &[&udp])),
        "ipv4_tcp" => Some(make("ipv4_tcp", &[&eth, &ipv4, &tcp])),
        "ipv4_udp" => Some(make("ipv4_udp", &[&eth, &ipv4, &udp])),
        _ => None,
    }
}

/// The built-in profile names (for `bpf-tcl profiles` / help).
#[must_use]
pub fn builtin_names() -> &'static [&'static str] {
    &["eth", "ipv4", "tcp", "udp", "ipv4_tcp", "ipv4_udp"]
}

/// Collect the (at most one) `profile` declaration from a file's top level.
///
/// # Errors
/// Bad arity, more than one profile, an unknown built-in, or a malformed field.
pub fn collect_profile(
    top_level: &Script,
    registry: &CommandRegistry,
) -> Result<Option<BpfProfileSpec>, BpfError> {
    let mut found: Option<BpfProfileSpec> = None;
    for stmt in &top_level.statements {
        if let Statement::Call {
            command,
            canonical_command,
            args,
            span,
            ..
        } = stmt
            && canonical_command.as_deref().unwrap_or(command.as_str()) == "profile"
        {
            if found.is_some() {
                return Err(BpfError::new(
                    BpfDiag::BadProfile,
                    *span,
                    "at most one `profile` per file (v1)",
                ));
            }
            found = Some(resolve_profile(args, *span, registry)?);
        }
    }
    Ok(found)
}

fn resolve_profile(
    args: &[String],
    span: Span,
    registry: &CommandRegistry,
) -> Result<BpfProfileSpec, BpfError> {
    match args.len() {
        // `profile NAME` — a built-in.
        1 => lookup_builtin(&args[0]).ok_or_else(|| {
            BpfError::new(
                BpfDiag::BadProfile,
                span,
                format!(
                    "unknown built-in profile `{}` (known: {})",
                    args[0],
                    builtin_names().join(", ")
                ),
            )
        }),
        // `profile NAME { field … }` — user-defined.
        2 => parse_user_profile(&args[0], &args[1], registry),
        _ => Err(BpfError::new(
            BpfDiag::BadProfile,
            span,
            "`profile` expects: profile NAME  or  profile NAME { field … }",
        )),
    }
}

fn parse_user_profile(
    name: &str,
    body: &str,
    registry: &CommandRegistry,
) -> Result<BpfProfileSpec, BpfError> {
    let module = lower_to_ir(body, registry);
    let mut fields = Vec::new();
    for stmt in &module.top_level.statements {
        if let Statement::Call {
            command,
            canonical_command,
            args,
            span,
            ..
        } = stmt
            && canonical_command.as_deref().unwrap_or(command.as_str()) == "field"
        {
            // field NAME OFFSET WIDTHBITS
            if args.len() != 3 {
                return Err(BpfError::new(
                    BpfDiag::BadProfile,
                    *span,
                    "`field` expects: field NAME OFFSET WIDTHBITS",
                ));
            }
            let offset = args[1].parse::<i32>().map_err(|_| {
                BpfError::new(BpfDiag::BadInt, *span, "field offset must be an integer")
            })?;
            let width_bits = parse_width(&args[2]).ok_or_else(|| {
                BpfError::new(
                    BpfDiag::BadProfile,
                    *span,
                    "field width must be 8, 16, or 32",
                )
            })?;
            fields.push(field(&args[0], offset, width_bits));
        }
    }
    Ok(BpfProfileSpec {
        name: name.to_owned(),
        fields,
    })
}

fn parse_width(s: &str) -> Option<u8> {
    match s.trim() {
        "8" => Some(8),
        "16" => Some(16),
        "32" => Some(32),
        _ => None,
    }
}

/// Rewrite a handler body's field-access commands into `load*` calls, recursing
/// into `if` bodies, and bind an implicit `__pkt` buffer if any field is read.
///
/// # Errors
/// A field command with the wrong arity.
pub fn expand_fields(script: &Script, profile: &BpfProfileSpec) -> Result<Script, BpfError> {
    let mut needs_pkt = false;
    let body = expand_script(script, profile, &mut needs_pkt)?;
    let mut stmts = body.statements;
    if needs_pkt {
        stmts.insert(0, synth_call("setbuf", &["__pkt", "ctx"], Span::empty(0)));
    }
    Ok(Script::from_statements(stmts))
}

fn expand_script(
    script: &Script,
    profile: &BpfProfileSpec,
    needs_pkt: &mut bool,
) -> Result<Script, BpfError> {
    let mut out = Vec::with_capacity(script.statements.len());
    for stmt in &script.statements {
        match stmt {
            Statement::Call {
                command,
                canonical_command,
                args,
                span,
                ..
            } => {
                let cmd = canonical_command.as_deref().unwrap_or(command.as_str());
                if let Some(f) = profile.field(cmd) {
                    if args.len() != 1 {
                        return Err(BpfError::new(
                            BpfDiag::BadArity,
                            *span,
                            format!("`{cmd}` (a `{}` field) expects: {cmd} DST", profile.name),
                        ));
                    }
                    *needs_pkt = true;
                    let load = match f.width_bits {
                        8 => "load8",
                        16 => "load16",
                        _ => "load32",
                    };
                    out.push(synth_call(
                        load,
                        &[&args[0], "__pkt", &f.offset.to_string()],
                        *span,
                    ));
                } else {
                    out.push(stmt.clone());
                }
            }
            Statement::If {
                span,
                clauses,
                else_body,
                else_span,
            } => {
                let clauses = clauses
                    .iter()
                    .map(|c| {
                        Ok(IfClause {
                            condition: c.condition.clone(),
                            condition_span: c.condition_span,
                            condition_base: c.condition_base,
                            body: expand_script(&c.body, profile, needs_pkt)?,
                            body_span: c.body_span,
                        })
                    })
                    .collect::<Result<Vec<_>, BpfError>>()?;
                let else_body = else_body
                    .as_ref()
                    .map(|b| expand_script(b, profile, needs_pkt))
                    .transpose()?;
                out.push(Statement::If {
                    span: *span,
                    clauses,
                    else_body,
                    else_span: *else_span,
                });
            }
            other => out.push(other.clone()),
        }
    }
    Ok(Script::from_statements(out))
}

/// Build a synthetic IR `Call` (used to splice expanded commands).
fn synth_call(cmd: &str, args: &[&str], span: Span) -> Statement {
    Statement::Call {
        span,
        command: cmd.to_owned(),
        canonical_command: None,
        args: args.iter().map(|s| (*s).to_owned()).collect(),
        defs: Vec::new(),
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: None,
        foreach_groups: None,
    }
}
