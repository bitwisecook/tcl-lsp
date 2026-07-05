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

//! Rename builtins (`category="rename"`): `rename`, `rename_partition`,
//! `rename_folder`, `rename_prefix`.
//!
//! `rename` queues a tolerant whole-source [`EditOp`](crate::edit_plan::EditOp)
//! that routes through [`rename_object`](crate::rewrite::rename_object); the
//! cascade builtins (`rename_partition` / `rename_folder` / `rename_prefix`)
//! queue token-bounded [`PrefixRewrite`](crate::edit_plan::PrefixRewrite)s.
//! All four are [`Builtin::Ctx`](super::Builtin::Ctx) — they need the parsed
//! config (for the active source URI and, for `rename`, the reference graph).
//!
//! Each returns the integer occurrence / match count.

use regex::Regex;
use tcl_bigip::value::{Folder, ObjectPath};

use crate::edit_plan::{After, Before, EditOp, PrefixRewrite};
use crate::errors::QueryError;
use crate::eval::EvalContext;
use crate::value::Value;

use super::{BuiltinSpec, as_str};

/// `regex::escape`-equivalent metacharacter escaping, used to build the
/// token-bounded cascade patterns.
fn re_escape(s: &str) -> String {
    regex::escape(s)
}

/// Count the token-bounded matches a [`PrefixRewrite`] will land on — the
/// builtin's return value (`len(pattern.findall(source))`). Reproduces the
/// `apply` engine's boundary filtering so the count is consistent.
fn count_matches(pattern: &Regex, before: Before, after: After, source: &str) -> usize {
    let bytes = source.as_bytes();
    let mut count = 0usize;
    let mut cursor = 0usize;
    while cursor <= source.len() {
        let Some(m) = pattern.find_at(source, cursor) else {
            break;
        };
        let (start, end) = (m.start(), m.end());
        let before_ok = match before {
            Before::Any => true,
            Before::NotIdent => start == 0 || !is_ident_byte(bytes[start - 1]),
        };
        let after_ok = match after {
            After::Any => true,
            After::RequireNameChar => {
                end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_')
            }
            After::NotIdent => end == bytes.len() || !is_ident_byte(bytes[end]),
        };
        if before_ok && after_ok {
            count += 1;
            cursor = if end > start { end } else { end + 1 };
        } else {
            cursor = start + 1;
        }
    }
    count
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'_' | b'/' | b'.' | b'-')
}

/// `rename(old, new)` — schedule a tolerant token-bounded source rewrite that
/// replaces every occurrence of `old` with `new`.
fn bi_rename(args: &[Value], ctx: &mut EvalContext) -> Result<Value, QueryError> {
    let old_s = as_str(&args[0], "rename", 1)?.trim().to_owned();
    let new_s = as_str(&args[1], "rename", 2)?.trim().to_owned();
    if old_s.is_empty() {
        return Err(QueryError::builtin("rename: old name must not be empty"));
    }
    if new_s.is_empty() {
        return Err(QueryError::builtin("rename: new name must not be empty"));
    }
    if old_s == new_s {
        return Ok(Value::Int(0));
    }

    // Partition-visibility check: when the rename changes the target's
    // partition, every existing referrer must still be able to see the new
    // partition. Refuse the move with an explicit list otherwise.
    check_partition_visibility(&old_s, &new_s, ctx)?;

    ctx.edits.add(EditOp {
        source_uri: ctx.root.uri.clone(),
        object_path: old_s,
        object_kind: String::new(),
        field_name: "name".to_owned(),
        operator: "=".to_owned(),
        new_value: Value::Str(new_s),
        field_slot: None,
        stanza_slot: None,
        strict: false,
    });
    Ok(Value::Int(1))
}

/// `rename_partition(old, new)` — rewrite every `/<old>/` prefix and the
/// `auth partition <old>` stanza header.
fn bi_rename_partition(args: &[Value], ctx: &mut EvalContext) -> Result<Value, QueryError> {
    let old_name = as_str(&args[0], "rename_partition", 1)?.trim().to_owned();
    let new_name = as_str(&args[1], "rename_partition", 2)?.trim().to_owned();
    if old_name.is_empty() || new_name.is_empty() {
        return Err(QueryError::builtin(
            "rename_partition: partition names must not be empty",
        ));
    }
    if old_name.contains('/') || new_name.contains('/') {
        return Err(QueryError::builtin(
            "rename_partition: pass bare partition names, not paths",
        ));
    }
    if !is_partition_name(&old_name) || !is_partition_name(&new_name) {
        return Err(QueryError::builtin(
            "rename_partition: partition names must match [A-Za-z0-9_.-]+",
        ));
    }
    if old_name == new_name {
        return Ok(Value::Int(0));
    }
    if old_name == "Common" {
        return Err(QueryError::builtin(
            "rename_partition: refusing to rename /Common — tenant partitions \
             reference /Common one-way (the F5 partition-visibility model), and \
             renaming it would silently break every cross-partition reference.  \
             Migrate the specific objects with rename(...) instead.",
        ));
    }
    if new_name == "Common" {
        return Err(QueryError::builtin(
            "rename_partition: refusing to rename a tenant partition to /Common \
             — /Common cannot reference tenant partitions, so any \
             cross-partition references in this config would be silently \
             invalidated.  Use check_partition_visibility() first to audit \
             existing references.",
        ));
    }

    // `/Old/...` -> `/New/...`: token-bounded so a longer name isn't matched.
    // Pattern: `(?<![A-Za-z0-9_/.\-])/Old/(?=[A-Za-z0-9_])` -> `/New/`.
    let prefix_re = compile(&format!(r"/{}/", re_escape(&old_name)))?;
    // `auth partition Old { ... }` — the standalone partition stanza.
    // Pattern: `(?<![…])(auth\s+partition\s+)Old(?![…])` -> `\g<1>New`.
    let header_re = compile(&format!(r"(auth\s+partition\s+){}", re_escape(&old_name)))?;

    let prefix_count = count_matches(
        &prefix_re,
        Before::NotIdent,
        After::RequireNameChar,
        &ctx.root.source,
    );
    let header_count = count_matches(
        &header_re,
        Before::NotIdent,
        After::NotIdent,
        &ctx.root.source,
    );

    ctx.edits.add_prefix(PrefixRewrite {
        source_uri: ctx.root.uri.clone(),
        label: format!("partition /{old_name}/"),
        pattern: prefix_re,
        before: Before::NotIdent,
        after: After::RequireNameChar,
        replacement: format!("/{new_name}/"),
        human_new: format!("/{new_name}/"),
    });
    ctx.edits.add_prefix(PrefixRewrite {
        source_uri: ctx.root.uri.clone(),
        label: format!("auth partition {old_name}"),
        pattern: header_re,
        before: Before::NotIdent,
        after: After::NotIdent,
        // The `\g<1>New`-style template; the regex crate uses `${1}`.
        replacement: format!("${{1}}{new_name}"),
        human_new: format!("auth partition {new_name}"),
    });

    Ok(Value::Int((prefix_count + header_count) as i64))
}

/// `rename_folder(old, new)` — rewrite every `<old>/` folder-path prefix.
fn bi_rename_folder(args: &[Value], ctx: &mut EvalContext) -> Result<Value, QueryError> {
    let old_text = as_str(&args[0], "rename_folder", 1)?.trim().to_owned();
    let new_text = as_str(&args[1], "rename_folder", 2)?.trim().to_owned();
    if old_text.is_empty() || new_text.is_empty() {
        return Err(QueryError::builtin(
            "rename_folder: folder paths must not be empty",
        ));
    }
    let Some(old_folder) = Folder::try_parse(&old_text) else {
        return Err(QueryError::builtin(format!(
            "rename_folder: cannot parse old folder {}",
            crate::eval::pyr_pub(&old_text)
        )));
    };
    let Some(new_folder) = Folder::try_parse(&new_text) else {
        return Err(QueryError::builtin(format!(
            "rename_folder: cannot parse new folder {}",
            crate::eval::pyr_pub(&new_text)
        )));
    };
    let old_canonical = old_folder.to_string();
    let new_canonical = new_folder.to_string();
    if old_canonical == new_canonical {
        return Ok(Value::Int(0));
    }

    // Pattern: `(?<![A-Za-z0-9_/.\-])Old/(?=[A-Za-z0-9_])` -> `New/`.
    let prefix_re = compile(&format!(r"{}/", re_escape(&old_canonical)))?;
    let count = count_matches(
        &prefix_re,
        Before::NotIdent,
        After::RequireNameChar,
        &ctx.root.source,
    );
    ctx.edits.add_prefix(PrefixRewrite {
        source_uri: ctx.root.uri.clone(),
        label: format!("folder {old_canonical}/"),
        pattern: prefix_re,
        before: Before::NotIdent,
        after: After::RequireNameChar,
        replacement: format!("{new_canonical}/"),
        human_new: format!("{new_canonical}/"),
    });
    Ok(Value::Int(count as i64))
}

/// `rename_prefix(old, new)` — rewrite every full-path occurrence beginning
/// with `old` to begin with `new`.
fn bi_rename_prefix(args: &[Value], ctx: &mut EvalContext) -> Result<Value, QueryError> {
    let old_text = as_str(&args[0], "rename_prefix", 1)?.trim().to_owned();
    let new_text = as_str(&args[1], "rename_prefix", 2)?.trim().to_owned();
    if old_text.is_empty() || new_text.is_empty() {
        return Err(QueryError::builtin(
            "rename_prefix: prefixes must not be empty",
        ));
    }
    if !old_text.starts_with('/') {
        return Err(QueryError::builtin(format!(
            "rename_prefix: old prefix must start with '/' (BIG-IP full paths) \
             — got {}.  Use ``rename`` for individual-object renames or \
             ``sub``/``gsub`` for free-form text rewrites.",
            crate::eval::pyr_pub(&old_text)
        )));
    }
    if !new_text.starts_with('/') {
        return Err(QueryError::builtin(format!(
            "rename_prefix: new prefix must start with '/' (BIG-IP full paths) — got {}.",
            crate::eval::pyr_pub(&new_text)
        )));
    }
    if old_text == new_text {
        return Ok(Value::Int(0));
    }
    // Pattern: `(?<![A-Za-z0-9_./\-])Old(?=[A-Za-z0-9_])` -> `New`. The
    // look-behind char set is the same identifier class.
    let prefix_re = compile(&re_escape(&old_text))?;
    let count = count_matches(
        &prefix_re,
        Before::NotIdent,
        After::RequireNameChar,
        &ctx.root.source,
    );
    ctx.edits.add_prefix(PrefixRewrite {
        source_uri: ctx.root.uri.clone(),
        label: format!("prefix {old_text}"),
        pattern: prefix_re,
        before: Before::NotIdent,
        after: After::RequireNameChar,
        replacement: new_text,
        human_new: String::new(),
    });
    Ok(Value::Int(count as i64))
}

/// Refuse a `rename` whose partition move would break visibility for any
/// existing referrer. A no-op when `old`/`new` aren't both parseable object
/// paths or share a partition.
fn check_partition_visibility(
    old_s: &str,
    new_s: &str,
    ctx: &EvalContext,
) -> Result<(), QueryError> {
    let (Some(old_p), Some(new_p)) = (ObjectPath::try_parse(old_s), ObjectPath::try_parse(new_s))
    else {
        return Ok(());
    };
    if old_p.partition() == new_p.partition() {
        return Ok(());
    }
    let mut broken: Vec<String> = Vec::new();
    for full_path in super::graph::reverse_referrers(old_s, &ctx.root) {
        if full_path == old_s {
            continue;
        }
        let Some(referrer) = ObjectPath::try_parse(&full_path) else {
            continue;
        };
        if !referrer.partition().can_see(new_p.partition()) {
            broken.push(full_path);
        }
    }
    if broken.is_empty() {
        return Ok(());
    }
    // De-duplicate + sort.
    broken.sort();
    broken.dedup();
    let total = broken.len();
    let shown = broken
        .iter()
        .take(10)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    let extra = if total > 10 {
        format!(" (+{} more)", total - 10)
    } else {
        String::new()
    };
    Err(QueryError::builtin(format!(
        "rename: moving {} to {} would break partition visibility for \
         {total} referrer(s): {shown}{extra}.  Move the referrer(s) to {} \
         first, drop the offending references, or pick a target partition \
         the referrers can see.",
        crate::eval::pyr_pub(old_s),
        crate::eval::pyr_pub(new_s),
        new_p.partition()
    )))
}

/// Whether `name` matches `[A-Za-z0-9_.-]+`.
fn is_partition_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
}

/// Compile a cascade *core* pattern (the pattern with its look-behind /
/// look-ahead boundaries stripped — those are carried out-of-band as
/// [`Before`] / [`After`]).
fn compile(core: &str) -> Result<Regex, QueryError> {
    Regex::new(core).map_err(|e| QueryError::builtin(format!("rename: invalid pattern: {e}")))
}

/// Registry entries for the rename builtins (all [`Builtin::Ctx`]).
pub(crate) fn registrations() -> Vec<(&'static str, BuiltinSpec)> {
    vec![
        super::ctx("rename", "rename", 2, Some(2), bi_rename),
        super::ctx(
            "rename_partition",
            "rename",
            2,
            Some(2),
            bi_rename_partition,
        ),
        super::ctx("rename_folder", "rename", 2, Some(2), bi_rename_folder),
        super::ctx("rename_prefix", "rename", 2, Some(2), bi_rename_prefix),
    ]
}
