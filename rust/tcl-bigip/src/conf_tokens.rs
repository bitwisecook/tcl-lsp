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

//! Lexical tokeniser for BIG-IP configuration text (`bigip.conf`, `.scf`),
//! for syntax highlighting.
//!
//! A `bigip.conf` is **not Tcl** — it is a brace-delimited declarative config.
//! Running the Tcl tokenizer over it reads each stanza body as one literal
//! braced word and emits whole *lines* as `string` tokens, which actively
//! mis-colours the file.  This is the config-mode analogue of
//! [`crate::apl::tokenise_apl`].
//!
//! # Non-overlapping by construction
//!
//! The LSP spec forbids overlapping semantic tokens.  Each line is lexed into
//! lexemes once and each lexeme classified once; a compound value (an object
//! path `/Common/web1:80`, an address with a route domain `10.0.0.1%2`) emits
//! **disjoint sub-tokens** rather than a specific token overlaid on a generic
//! one.
//!
//! Overlaying is the failure mode this design rules out: appending BIG-IP
//! matches on top of tokens a generic Tcl collector has already produced
//! yields overlapping pairs — `'Common':string` *and* `'Common':partition`
//! on the same span.  It must not come back.

use std::sync::LazyLock;

use regex::Regex;

/// Semantic classification of a BIG-IP config lexeme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BigipTokenKind {
    /// A `#` comment line.
    Comment,
    /// A module (`ltm`, `net`, `auth`, …) or object-type (`pool`, `node`, …)
    /// word in a stanza header.
    Keyword,
    /// A property key inside a stanza body (`address`, `interval`, `monitor`).
    Property,
    /// A partition name (the first component of a `/Common/…` path).
    Partition,
    /// A generic object name — the tail of an object path.
    Object,
    /// An object name known to be a pool (`pool`, `snatpool`, `last-hop-pool`).
    Pool,
    /// An object name known to be a monitor.
    Monitor,
    /// An object name known to be a profile.
    Profile,
    /// An object name known to be a VLAN or trunk.
    Vlan,
    /// A BIG-IP network interface name (`1.1`, `mgmt`).
    Interface,
    /// An IPv4 literal, with optional CIDR suffix.
    IpAddress,
    /// A TCP/UDP port number.
    Port,
    /// A route domain (`%0`).
    RouteDomain,
    /// A fully-qualified domain name.
    Fqdn,
    /// A user name.
    Username,
    /// An encrypted / secret value.
    Encrypted,
    /// A quoted-string fragment.
    Str,
    /// A backslash escape inside a string.
    Escape,
    /// A bare numeric literal.
    Number,
}

/// A single lexical token, as absolute byte offsets into the source.  `end` is
/// exclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BigipToken {
    /// Absolute byte offset of the first byte.
    pub start: u32,
    /// Absolute byte offset one past the last byte.
    pub end: u32,
    /// What the lexeme is.
    pub kind: BigipTokenKind,
}

/// The tmsh top-level modules that open a stanza.
const MODULES: &[&str] = &[
    "apm", "asm", "auth", "cli", "cm", "gtm", "ilx", "ltm", "net", "pem", "security", "sys", "wom",
];

/// Property keys whose value names an object of a known kind.
fn keyed_ref_kind(key: &str) -> Option<BigipTokenKind> {
    Some(match key {
        "pool" | "last-hop-pool" | "snatpool" | "default-pool" => BigipTokenKind::Pool,
        "monitor" | "monitors" => BigipTokenKind::Monitor,
        "profile" | "profiles" | "defaults-from" => BigipTokenKind::Profile,
        "vlan" | "vlans" | "vlan-group" => BigipTokenKind::Vlan,
        "rule"
        | "rules"
        | "policy"
        | "policies"
        | "persist"
        | "fallback-persistence"
        | "snat-translation"
        | "traffic-group"
        | "device-group"
        | "gateway"
        | "destination"
        | "virtual-address"
        | "route-domain"
        | "default-route-domain"
        | "node"
        | "members" => BigipTokenKind::Object,
        _ => return None,
    })
}

/// The object-type word in a stanza header → the type its *name* should carry.
fn decl_kind(object_type: &str) -> BigipTokenKind {
    match object_type {
        "pool" | "snatpool" => BigipTokenKind::Pool,
        "monitor" => BigipTokenKind::Monitor,
        "profile" => BigipTokenKind::Profile,
        "vlan" | "trunk" => BigipTokenKind::Vlan,
        "interface" => BigipTokenKind::Interface,
        _ => BigipTokenKind::Object,
    }
}

/// Keys whose value is a secret.
const SECRET_KEYS: &[&str] = &[
    "secret",
    "passphrase",
    "password",
    "encrypted-password",
    "encrypted",
];

/// Keys whose value is a user name.
const USER_KEYS: &[&str] = &["user", "username"];

/// Filesystem paths are not object paths — `/config/…` is a file, not
/// `/Partition/Object`.
const FS_PREFIXES: &[&str] = &["/config/", "/usr/", "/var/", "/shared/", "/etc/"];

/// IPv4 with an optional route domain and CIDR suffix.
static IPV4_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(?:(?:25[0-5]|2[0-4]\d|1?\d?\d)\.){3}(?:25[0-5]|2[0-4]\d|1?\d?\d)(%\d+)?(/\d{1,2})?$",
    )
    .expect("BIG-IP IPv4 pattern is valid")
});

/// A BIG-IP interface name: `1.1`, `2.3`, or `mgmt`.
static INTERFACE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:\d+\.\d+|mgmt)$").expect("BIG-IP interface pattern is valid")
});

/// A bare integer.
static NUMBER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\d+$").expect("BIG-IP number pattern is valid"));

/// One lexeme of a line, as absolute byte offsets.
#[derive(Debug, Clone, Copy)]
struct Lexeme {
    start: usize,
    end: usize,
    class: LexClass,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LexClass {
    /// A bare word (any run of non-space, non-brace, non-quote bytes).
    Word,
    /// A `"…"` quoted string, including both delimiters.
    Quoted,
    /// `{` or `}`.
    Brace,
}

/// The byte spans of the **embedded iRule bodies** in `source` — the content of
/// each `ltm rule /Common/name { … }` stanza, between its `{` and its matching
/// `}`.
///
/// A rule body is Tcl, not config: it must be handed to the Tcl/iRules
/// tokenizer, not read as `key value` property lines.  Without this its `when` /
/// `if` / `switch` came out as config *property keys* and its `[HTTP::uri]`
/// substitutions were not tokenised at all — config highlighting applied to
/// code, which is worse than none.
///
/// [`tokenise_bigip_conf`] emits nothing inside these spans, leaving them for
/// the caller to walk as Tcl.
#[must_use]
pub fn embedded_rule_bodies(source: &str) -> Vec<(u32, u32)> {
    let mut spans = Vec::new();
    let mut line_start = 0usize;
    let mut depth: i32 = 0;
    // The depth a rule stanza opened at, and where its body content began.
    let mut open: Option<(i32, usize)> = None;

    for line in source.split('\n') {
        let trimmed = line.trim_start();
        if !trimmed.starts_with('#') {
            let lexemes = lex_line(line, line_start);
            let words: Vec<&Lexeme> = lexemes
                .iter()
                .filter(|l| l.class != LexClass::Brace)
                .collect();
            // `<module> rule <path> {` at top level opens an embedded iRule.
            let is_rule = depth == 0
                && open.is_none()
                && words.len() >= 2
                && MODULES.contains(&&source[words[0].start..words[0].end])
                && &source[words[1].start..words[1].end] == "rule";

            for lx in &lexemes {
                if lx.class != LexClass::Brace {
                    continue;
                }
                if &source[lx.start..lx.end] == "{" {
                    if is_rule && open.is_none() {
                        open = Some((depth, lx.end));
                    }
                    depth += 1;
                } else {
                    depth = (depth - 1).max(0);
                    if let Some((odepth, body_start)) = open
                        && depth == odepth
                    {
                        spans.push((
                            u32::try_from(body_start).unwrap_or(0),
                            u32::try_from(lx.start).unwrap_or(0),
                        ));
                        open = None;
                    }
                }
            }
        }
        line_start += line.len() + 1;
    }
    spans
}

/// Tokenise BIG-IP config text for highlighting.
///
/// The returned tokens are sorted by `start` and are guaranteed
/// **non-overlapping**.  Embedded iRule bodies (see [`embedded_rule_bodies`])
/// are left empty for the caller to walk as Tcl.
#[must_use]
pub fn tokenise_bigip_conf(source: &str) -> Vec<BigipToken> {
    let rule_bodies = embedded_rule_bodies(source);
    let mut out: Vec<BigipToken> = Vec::new();
    let mut line_start = 0usize;
    // The kind implied for the *direct children* of each open block, innermost
    // last.  A list block inherits its key's kind, so the bare members of
    // `vlans { /Common/internal … }` are VLANs rather than generic objects —
    // the key is on the opening line, the values on the following ones.  Depth
    // is `stack.len() - 1` (the base entry is the file itself, depth 0).
    let mut stack: Vec<Option<BigipTokenKind>> = vec![None];

    for line in source.split('\n') {
        let depth = i32::try_from(stack.len()).unwrap_or(1) - 1;
        let inherited = stack.last().copied().flatten();
        let (lexemes, opened) = tokenise_line(source, line, line_start, depth, inherited, &mut out);
        // The first `{` on the line opens the block the line's key described;
        // any further `{` is a nested record, which inherits nothing.
        let mut first_open = true;
        for lx in &lexemes {
            if lx.class != LexClass::Brace {
                continue;
            }
            if &source[lx.start..lx.end] == "{" {
                stack.push(if first_open { opened } else { None });
                first_open = false;
            } else if stack.len() > 1 {
                stack.pop();
            }
        }
        line_start += line.len() + 1;
    }

    // Drop the tokens that land *inside* an embedded iRule body — per token, not
    // per line.  A body need not own whole lines: `ltm rule /Common/r { when X {
    // pool /Common/p } }` puts config text and iRules code on one line, and a
    // whole-line test then config-lexed the code *while* the caller also walked
    // it as Tcl, producing overlapping tokens (`'when':keyword` over
    // `'when':property`) — the one thing these lexers exist to prevent.  Braces
    // are their own lexemes, so no token can straddle a body boundary and this
    // containment test is exact.
    out.retain(|t| !rule_bodies.iter().any(|&(s, e)| t.start >= s && t.end <= e));

    debug_assert!(
        out.windows(2).all(|w| w[0].end <= w[1].start),
        "BIG-IP tokens must be sorted and non-overlapping"
    );
    out
}

/// Tokenise one line, appending to `out`; returns its lexemes so the caller can
/// update the brace depth.
fn tokenise_line(
    source: &str,
    line: &str,
    base: usize,
    depth: i32,
    inherited: Option<BigipTokenKind>,
    out: &mut Vec<BigipToken>,
) -> (Vec<Lexeme>, Option<BigipTokenKind>) {
    let trimmed = line.trim_start();
    if trimmed.is_empty() {
        return (Vec::new(), None);
    }

    // A `#` line is a comment whole, and no other rule runs on it.
    if trimmed.starts_with('#') {
        let indent = line.len() - trimmed.len();
        push(
            out,
            base + indent,
            base + line.trim_end().len(),
            BigipTokenKind::Comment,
        );
        return (Vec::new(), None);
    }

    let lexemes = lex_line(line, base);
    let words: Vec<Lexeme> = lexemes
        .iter()
        .copied()
        .filter(|l| l.class != LexClass::Brace)
        .collect();
    if words.is_empty() {
        return (lexemes, None);
    }

    let text = |l: &Lexeme| &source[l.start..l.end];
    let head = text(&words[0]);

    // A stanza header lives at depth 0 and opens with a tmsh module word.
    let opened = if depth == 0 && MODULES.contains(&head) {
        emit_header(source, &words, out);
        None
    } else {
        emit_property(source, &words, inherited, out)
    };
    (lexemes, opened)
}

/// `ltm pool /Common/web_pool {`, `auth partition foo {`, `auth user admin {`.
fn emit_header(source: &str, words: &[Lexeme], out: &mut Vec<BigipToken>) {
    let text = |l: &Lexeme| &source[l.start..l.end];
    push(out, words[0].start, words[0].end, BigipTokenKind::Keyword);

    // The object-type words run until the name (a `/path` or the final bare
    // word).  `ltm monitor http /Common/m` and `ltm data-group internal /C/d`
    // both have two type words; `ltm pool /Common/p` has one.
    let name_idx = words
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, l)| text(l).starts_with('/'))
        .map_or(words.len().saturating_sub(1), |(i, _)| i);

    let mut kind = BigipTokenKind::Object;
    for (i, w) in words.iter().enumerate().take(name_idx).skip(1) {
        push(out, w.start, w.end, BigipTokenKind::Keyword);
        // The *first* type word decides what the name is.
        if i == 1 {
            kind = decl_kind(text(w));
        }
    }

    let Some(name) = words.get(name_idx) else {
        return;
    };
    if name_idx == 0 {
        return;
    }
    let name_text = text(name);
    // `auth partition foo` / `auth user admin` name a partition / user
    // directly rather than by path.
    let second = words.get(1).map_or("", text);
    if !name_text.starts_with('/') {
        let bare = match second {
            "partition" => BigipTokenKind::Partition,
            "user" => BigipTokenKind::Username,
            _ if INTERFACE_RE.is_match(name_text) => BigipTokenKind::Interface,
            _ => return,
        };
        push(out, name.start, name.end, bare);
        return;
    }
    emit_path(source, *name, kind, out);
}

/// A property line inside a stanza body, or a bare value inside a nested block.
///
/// Returns the kind this line's key implies for a block it opens, so the bare
/// members of `vlans { … }` inherit `Vlan`.
fn emit_property(
    source: &str,
    words: &[Lexeme],
    inherited: Option<BigipTokenKind>,
    out: &mut Vec<BigipToken>,
) -> Option<BigipTokenKind> {
    let text = |l: &Lexeme| &source[l.start..l.end];
    let key_lx = words[0];
    let key = text(&key_lx);

    // Not every line inside a body is `key value`.  A data-group record
    // (`www.example.com { }`, `/old-path { … }`, `10.0.0.0/8 { … }`) and a bare
    // member of a list block (`vlans { /Common/internal }`) are *values* in the
    // key position.  Only an identifier-shaped head word is a property key —
    // anything carrying a `/`, `.`, `:` or `%` is a value, and takes the kind
    // the enclosing block implies.
    if !is_key_like(key) {
        for w in words {
            emit_value(source, *w, inherited, out);
        }
        return None;
    }

    push(out, key_lx.start, key_lx.end, BigipTokenKind::Property);

    // What the key implies about its value.
    let implied = if SECRET_KEYS.contains(&key) {
        Some(BigipTokenKind::Encrypted)
    } else if USER_KEYS.contains(&key) {
        Some(BigipTokenKind::Username)
    } else if key.contains("port") {
        Some(BigipTokenKind::Port)
    } else {
        keyed_ref_kind(key)
    };

    for w in &words[1..] {
        emit_value(source, *w, implied, out);
    }
    implied
}

/// Classify one value lexeme.  `implied` is the type the property key implies
/// for its value, if any.
fn emit_value(
    source: &str,
    lx: Lexeme,
    implied: Option<BigipTokenKind>,
    out: &mut Vec<BigipToken>,
) {
    let text = &source[lx.start..lx.end];

    if lx.class == LexClass::Quoted {
        push_string(source, lx.start, lx.end, out);
        return;
    }

    // An object path: `/Common/web1:80`, `/Common/http`.
    if text.starts_with('/') && !FS_PREFIXES.iter().any(|p| text.starts_with(p)) {
        emit_path(source, lx, implied.unwrap_or(BigipTokenKind::Object), out);
        return;
    }

    // An address, possibly with a route domain and/or CIDR.
    if let Some(caps) = IPV4_RE.captures(text) {
        let rd = caps.get(1).map(|m| (m.start(), m.end()));
        match rd {
            Some((s, e)) => {
                push(out, lx.start, lx.start + s, BigipTokenKind::IpAddress);
                push(out, lx.start + s, lx.start + e, BigipTokenKind::RouteDomain);
                // The `/24` tail, if any, follows the route domain.
                if e < text.len() {
                    push(out, lx.start + e, lx.end, BigipTokenKind::IpAddress);
                }
            }
            None => push(out, lx.start, lx.end, BigipTokenKind::IpAddress),
        }
        return;
    }

    // A `proto:port` / `host:port` value (`tcp:22`, `www.example.com:443`):
    // split the port off rather than overlaying it, and classify the head.
    if let Some((head, port)) = text.rsplit_once(':')
        && !port.is_empty()
        && port.bytes().all(|b| b.is_ascii_digit())
        && !head.is_empty()
    {
        let head_lx = Lexeme {
            start: lx.start,
            end: lx.start + head.len(),
            class: LexClass::Word,
        };
        emit_value(source, head_lx, implied, out);
        let port_at = lx.start + head.len() + 1;
        push(out, port_at, port_at + port.len(), BigipTokenKind::Port);
        return;
    }

    if let Some(kind) = implied {
        // A port key's value is a port; a secret key's value is a secret; a
        // `pool`/`monitor` key's bare (unpathed) value still names that object.
        push(out, lx.start, lx.end, kind);
        return;
    }

    if NUMBER_RE.is_match(text) {
        push(out, lx.start, lx.end, BigipTokenKind::Number);
        return;
    }
    if INTERFACE_RE.is_match(text) {
        push(out, lx.start, lx.end, BigipTokenKind::Interface);
        return;
    }
    if looks_like_fqdn(text) {
        push(out, lx.start, lx.end, BigipTokenKind::Fqdn);
    }
}

/// Whether `text` is shaped like a property key rather than a value: a bare
/// identifier (letters, digits, `_`, `-`).  A `/`, `.`, `:` or `%` makes it a
/// path / FQDN / address / route domain — i.e. a value.
fn is_key_like(text: &str) -> bool {
    !text.is_empty()
        && text.starts_with(|c: char| c.is_ascii_alphabetic())
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Emit the disjoint sub-tokens of an object path: `/Common/web1:80` →
/// `Common`:Partition, `web1`:<tail>, `80`:Port.  The `/` and `:` separators
/// are deliberately left uncovered.
fn emit_path(source: &str, lx: Lexeme, tail: BigipTokenKind, out: &mut Vec<BigipToken>) {
    let text = &source[lx.start..lx.end];
    let mut off = 1usize; // past the leading '/'
    let body = &text[1..];
    let mut parts = body.split('/').peekable();

    // The first component is the partition, but only when a name follows it.
    // A single-component path (a data-group record key like `/old-path`) is a
    // name, not a partition.
    if let Some(first) = parts.next() {
        if parts.peek().is_some() {
            push(
                out,
                lx.start + off,
                lx.start + off + first.len(),
                BigipTokenKind::Partition,
            );
        } else {
            push(out, lx.start + off, lx.start + off + first.len(), tail);
        }
        off += first.len() + 1;
    }

    // Everything after is the object name — the last component carries the
    // type; a `:port` suffix on it splits off as a Port.
    let rest: Vec<&str> = parts.collect();
    for (i, part) in rest.iter().enumerate() {
        let last = i + 1 == rest.len();
        let kind = if last { tail } else { BigipTokenKind::Object };
        match part.split_once(':') {
            Some((name, port)) if last && !port.is_empty() => {
                push(out, lx.start + off, lx.start + off + name.len(), kind);
                let port_at = lx.start + off + name.len() + 1;
                push(out, port_at, port_at + port.len(), BigipTokenKind::Port);
            }
            _ => push(out, lx.start + off, lx.start + off + part.len(), kind),
        }
        off += part.len() + 1;
    }
}

/// Emit a quoted string as `Str` fragments split around its escapes.
fn push_string(source: &str, start: usize, end: usize, out: &mut Vec<BigipToken>) {
    let text = &source[start..end];
    for seg in tcl_lexer::split_backslash_escapes(text) {
        let kind = if seg.is_escape {
            BigipTokenKind::Escape
        } else {
            BigipTokenKind::Str
        };
        push(out, start + seg.start, start + seg.end, kind);
    }
}

/// Heuristic FQDN test: at least two dot-separated labels, at least one letter,
/// and every label a valid hostname label.  Rejects bare numbers and addresses.
fn looks_like_fqdn(text: &str) -> bool {
    let atom = text.trim_end_matches('.');
    if !atom.contains('.') || !atom.chars().any(char::is_alphabetic) {
        return false;
    }
    let labels: Vec<&str> = atom.split('.').collect();
    labels.len() >= 2
        && labels.iter().all(|l| {
            !l.is_empty()
                && !l.starts_with('-')
                && !l.ends_with('-')
                && l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        })
}

/// Split a line into lexemes.  Offsets are absolute (`base` + position).
fn lex_line(line: &str, base: usize) -> Vec<Lexeme> {
    let bytes = line.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;

    while i < bytes.len() {
        if bytes[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }
        let start = i;
        let class = match bytes[i] {
            b'"' => {
                i += 1;
                while i < bytes.len() {
                    match bytes[i] {
                        b'\\' if i + 1 < bytes.len() => i += 2,
                        b'"' => {
                            i += 1;
                            break;
                        }
                        _ => i += 1,
                    }
                }
                LexClass::Quoted
            }
            b'{' | b'}' => {
                i += 1;
                LexClass::Brace
            }
            _ => {
                while i < bytes.len()
                    && !bytes[i].is_ascii_whitespace()
                    && !matches!(bytes[i], b'{' | b'}' | b'"')
                {
                    i += 1;
                }
                LexClass::Word
            }
        };
        out.push(Lexeme {
            start: base + start,
            end: base + i,
            class,
        });
    }
    out
}

/// Append a token, skipping empty spans.
fn push(out: &mut Vec<BigipToken>, start: usize, end: usize, kind: BigipTokenKind) {
    if end <= start {
        return;
    }
    out.push(BigipToken {
        start: u32::try_from(start).unwrap_or(u32::MAX),
        end: u32::try_from(end).unwrap_or(u32::MAX),
        kind,
    });
}

#[cfg(test)]
mod tests {
    use super::BigipTokenKind as K;
    use super::*;

    fn toks(src: &str) -> Vec<(&str, K)> {
        tokenise_bigip_conf(src)
            .into_iter()
            .map(|t| (&src[t.start as usize..t.end as usize], t.kind))
            .collect()
    }

    fn assert_no_overlap(src: &str) {
        let t = tokenise_bigip_conf(src);
        for w in t.windows(2) {
            assert!(
                w[0].end <= w[1].start,
                "overlapping tokens: {:?} then {:?}",
                w[0],
                w[1]
            );
        }
    }

    #[test]
    fn pool_header_splits_partition_and_name() {
        assert_eq!(
            toks("ltm pool /Common/web_pool {\n}\n"),
            vec![
                ("ltm", K::Keyword),
                ("pool", K::Keyword),
                ("Common", K::Partition),
                ("web_pool", K::Pool),
            ]
        );
        assert_no_overlap("ltm pool /Common/web_pool {\n}\n");
    }

    /// The separators are deliberately uncovered — that is what keeps the
    /// sub-tokens disjoint instead of overlaying a whole-path token.
    #[test]
    fn member_path_splits_partition_name_and_port() {
        assert_eq!(
            toks(
                "ltm pool /C/p {\n    members {\n        /Common/web1:80 {\n        }\n    }\n}\n"
            ),
            vec![
                ("ltm", K::Keyword),
                ("pool", K::Keyword),
                ("C", K::Partition),
                ("p", K::Pool),
                ("members", K::Property),
                ("Common", K::Partition),
                ("web1", K::Object),
                ("80", K::Port),
            ]
        );
    }

    #[test]
    fn two_word_object_types() {
        assert_eq!(
            toks("ltm monitor http /Common/my_mon {\n}\n"),
            vec![
                ("ltm", K::Keyword),
                ("monitor", K::Keyword),
                ("http", K::Keyword),
                ("Common", K::Partition),
                ("my_mon", K::Monitor),
            ]
        );
    }

    #[test]
    fn partition_and_user_declarations() {
        assert_eq!(
            toks("auth partition foo {\n}\n"),
            vec![
                ("auth", K::Keyword),
                ("partition", K::Keyword),
                ("foo", K::Partition),
            ]
        );
        assert_eq!(
            toks("auth user admin {\n}\n"),
            vec![
                ("auth", K::Keyword),
                ("user", K::Keyword),
                ("admin", K::Username),
            ]
        );
    }

    #[test]
    fn addresses_ports_and_route_domains() {
        assert_eq!(
            toks("ltm node /C/n {\n    address 10.0.1.10\n}\n"),
            vec![
                ("ltm", K::Keyword),
                ("node", K::Keyword),
                ("C", K::Partition),
                ("n", K::Object),
                ("address", K::Property),
                ("10.0.1.10", K::IpAddress),
            ]
        );
        // A route domain splits off the address rather than overlaying it.
        let t = toks("ltm x /C/n {\n    address 10.0.0.1%2\n}\n");
        assert!(t.contains(&("10.0.0.1", K::IpAddress)), "{t:?}");
        assert!(t.contains(&("%2", K::RouteDomain)), "{t:?}");
        assert_no_overlap("ltm x /C/n {\n    address 10.0.0.1%2\n}\n");
    }

    #[test]
    fn cidr_is_part_of_the_address() {
        let t = toks("ltm x /C/n {\n    address 10.0.0.0/8\n}\n");
        assert!(t.contains(&("10.0.0.0/8", K::IpAddress)), "{t:?}");
    }

    #[test]
    fn keyed_object_references_take_the_key_s_kind() {
        let t =
            toks("ltm virtual /C/v {\n    pool /Common/web_pool\n    monitor /Common/http\n}\n");
        assert!(t.contains(&("web_pool", K::Pool)), "{t:?}");
        assert!(t.contains(&("http", K::Monitor)), "{t:?}");
        assert!(t.contains(&("pool", K::Property)), "{t:?}");
    }

    #[test]
    fn secrets_and_usernames() {
        let t = toks("auth radius-server /C/r {\n    secret $M$abc=\n    server 10.0.0.1\n}\n");
        assert!(t.contains(&("$M$abc=", K::Encrypted)), "{t:?}");
        assert!(t.contains(&("10.0.0.1", K::IpAddress)), "{t:?}");
    }

    #[test]
    fn fqdn_records_and_quoted_values() {
        let t = toks(
            "ltm data-group internal /C/d {\n    records {\n        www.example.com { }\n    }\n}\n",
        );
        assert!(t.contains(&("www.example.com", K::Fqdn)), "{t:?}");
        let q = toks("ltm x /C/m {\n    recv \"200 OK\"\n}\n");
        assert!(q.contains(&("\"200 OK\"", K::Str)), "{q:?}");
    }

    /// An escape inside a quoted value splits the string rather than
    /// overlaying it.
    #[test]
    fn escapes_split_quoted_values() {
        let t = toks("ltm x /C/m {\n    send \"GET /h\\r\\n\"\n}\n");
        assert!(t.contains(&(r"\r", K::Escape)), "{t:?}");
        assert_no_overlap("ltm x /C/m {\n    send \"GET /h\\r\\n\"\n}\n");
    }

    /// A filesystem path is not an object path: `/config/ssl/cert.crt` must not
    /// be split into a `config` partition and an `ssl` object.  (The stanza
    /// header's own `/C/s` *is* an object path, and still resolves.)
    #[test]
    fn filesystem_paths_are_not_object_paths() {
        let t = toks("sys x /C/s {\n    file /config/ssl/cert.crt\n}\n");
        assert!(
            !t.iter()
                .any(|(text, _)| *text == "config" || *text == "ssl"),
            "a /config/… path must not be split into partition/object: {t:?}"
        );
        assert!(t.contains(&("C", K::Partition)), "{t:?}");
    }

    #[test]
    fn comments_are_whole_lines() {
        assert_eq!(toks("# a comment\n"), vec![("# a comment", K::Comment)]);
    }

    /// A list block's bare members inherit the kind its key implies — the key
    /// is on the opening line, the values on the lines below it.
    #[test]
    fn list_block_members_inherit_the_key_s_kind() {
        let t = toks(
            "net route-domain /Common/0 {\n    vlans {\n        /Common/internal\n        /Common/external\n    }\n}\n",
        );
        assert!(t.contains(&("internal", K::Vlan)), "{t:?}");
        assert!(t.contains(&("external", K::Vlan)), "{t:?}");
        assert!(t.contains(&("vlans", K::Property)), "{t:?}");
    }

    /// A `proto:port` value splits rather than overlaying.
    #[test]
    fn proto_port_values_split_off_the_port() {
        let t = toks("net self-allow {\n    defaults {\n        tcp:22\n    }\n}\n");
        assert!(t.contains(&("22", K::Port)), "{t:?}");
        assert_no_overlap("net self-allow {\n    defaults {\n        tcp:22\n    }\n}\n");
    }

    #[test]
    fn whole_sample_files_have_no_overlaps() {
        for src in [
            include_str!("../../../samples/bigip/bigip.conf"),
            include_str!("../../../samples/bigip/bigip_base.conf"),
        ] {
            assert_no_overlap(src);
            assert!(
                tokenise_bigip_conf(src).len() > 100,
                "expected a substantial token stream"
            );
        }
    }

    /// A `ltm rule { … }` body is iRules code, not config: the config lexer must
    /// leave it alone so the caller can walk it as Tcl.  Read as config, its
    /// `when` / `if` / `switch` became config *property keys*.
    #[test]
    fn embedded_rule_bodies_are_left_for_the_tcl_walker() {
        let src = "ltm rule /Common/r {\n    when HTTP_REQUEST {\n        pool /Common/p\n    }\n}\nltm pool /Common/q {\n    monitor /Common/http\n}\n";
        let bodies = embedded_rule_bodies(src);
        assert_eq!(bodies.len(), 1, "one rule body: {bodies:?}");
        let (s0, e0) = bodies[0];
        let body = &src[s0 as usize..e0 as usize];
        assert!(body.contains("when HTTP_REQUEST"), "body: {body:?}");
        assert!(!body.contains("ltm pool"), "the body must stop at its `}}`");

        // Nothing is emitted inside the body …
        let t = toks(src);
        assert!(
            !t.iter()
                .any(|(text, _)| *text == "when" || *text == "HTTP_REQUEST"),
            "the config lexer must not tokenise rule-body Tcl: {t:?}"
        );
        // … while the stanza header and the *following* stanza still tokenise.
        assert!(t.contains(&("rule", K::Keyword)), "{t:?}");
        assert!(
            t.contains(&("q", K::Pool)),
            "the next stanza must resume: {t:?}"
        );
        assert!(t.contains(&("http", K::Monitor)), "{t:?}");
        assert_no_overlap(src);
    }

    /// A rule body need not own whole lines.  `ltm rule /Common/r { when X { pool
    /// /Common/p } }` puts config text and iRules code on one line; a whole-line
    /// suppression test then config-lexed the code *while* the caller also walked
    /// it as Tcl, yielding overlapping tokens (`'when':keyword` over
    /// `'when':property`).  Suppression is per *token*, so this cannot recur.
    #[test]
    fn partial_line_rule_bodies_are_not_config_lexed() {
        for src in [
            // Body content sharing the closing-brace line.
            "ltm rule /Common/r {\n    when HTTP_REQUEST { pool /Common/p } }\n",
            // Whole stanza on one line.
            "ltm rule /Common/r { when HTTP_REQUEST { pool /Common/p } }\n",
        ] {
            assert_no_overlap(src);
            let t = toks(src);
            // The header still tokenises …
            assert!(t.contains(&("rule", K::Keyword)), "{src:?} -> {t:?}");
            // … and nothing inside the body does.
            for word in ["when", "HTTP_REQUEST", "pool"] {
                assert!(
                    !t.iter().any(|(text, _)| *text == word),
                    "`{word}` is rule-body Tcl and must not be config-lexed: {t:?}"
                );
            }
        }
    }

    /// Tcl escape widths are not uniform, so the lexer cannot assume every
    /// escape is two bytes — that would split `\x41` into an escape `\x` plus
    /// a string `41`.
    #[test]
    fn escape_widths_follow_tcl() {
        let t = toks("ltm x /C/m {\n    recv \"a\\x41b\\U0001F600c\\101d\"\n}\n");
        for esc in ["\\x41", "\\U0001F600", "\\101"] {
            assert!(
                t.contains(&(esc, K::Escape)),
                "`{esc}` must be one escape token: {t:?}"
            );
        }
    }

    #[test]
    fn non_ascii_offsets_are_byte_exact() {
        let src = "ltm pool /Common/p {\n    description \"café\"\n}\n";
        for t in tokenise_bigip_conf(src) {
            assert!(
                src.is_char_boundary(t.start as usize) && src.is_char_boundary(t.end as usize),
                "token {t:?} must land on char boundaries"
            );
        }
    }
}
