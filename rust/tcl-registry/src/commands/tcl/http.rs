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

//! `http` — Tcl's bundled HTTP/1.x client package (`package require http`).
//!
//! Registers the bare `http` identifier: the word that appears as the
//! argument to `package require http`, and the anchor for its
//! package-level hover summary. There is no real invokable command named
//! plain `http` — every actual entry point (`::http::geturl`,
//! `::http::config`, `::http::wait`, …) is a separate namespace-qualified
//! proc, each with its own `CommandSpec` under
//! `commands/stdlib/http__*.rs`.

use crate::prelude::*;

/// Command spec for the `http` package identifier.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http",
        // Bundled with every standard Tcl release 8.4 through 9.1 — the
        // command set it provides only grows across versions (see the
        // hover snippet below), never disappears — so this stays
        // universal rather than version-gated. F5 iRules removes the
        // whole package from its TMM sandbox (the K36322151 bans); that
        // is modelled as a subtractive exclusion keyed on the literal
        // name "http" in `tcl-dialect/src/profile.rs`'s
        // `IRULES_DISABLED_COMMANDS`, alongside `open`/`socket`/`file` —
        // which, like this spec, all keep `dialects: None` and rely on
        // that list rather than a `dialects` restriction here.
        dialects: None,
        // Never actually invoked as a command (see the module doc
        // comment) — left unconstrained rather than modelling an
        // invocation shape that doesn't exist.
        arity: Arity::any(),
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        hover: Some(HoverSnippet {
            summary: "Client-side implementation of the HTTP/1.1 protocol, built around http::geturl and a per-transaction state token.",
            synopsis: &["package require http ?version?"],
            snippet: "`::http::geturl` is the primary entry point: it runs an HTTP transaction and returns a token naming a per-transaction state array, queried through `::http::data`, `::http::code`, `::http::status`, `::http::error`, `::http::size`, and `::http::meta` (always finish with `::http::cleanup` on the token to release it). `::http::config` sets process-wide defaults such as the proxy host/port and User-Agent. `::http::register` adds custom transports such as HTTPS. Tcl 8.4's http package implements HTTP/1.0 only; HTTP/1.1 support (persistent connections via `-keepalive`, plus `-protocol`, `-method`, and `-strict` URL validation on `::http::geturl`) arrived in 8.5, and 8.6 added pipelining tunables to `::http::config` (`-pipeline`, `-postfresh`, `-repost`, `-zip`) plus `::http::quoteString` and `::http::registerError`. Tcl 9.0 added a richer response-introspection layer — `::http::responseInfo`, `::http::requestLine`, `::http::responseHeaders`/`::http::responseHeaderValue`, `::http::reasonPhrase`, `::http::postError`, and a `-cookiejar` hook on `::http::config` — alongside the older `::http::code`/`::http::data`/`::http::meta`/`::http::ncode`, which 9.0+ documents as aliases for the new names. Not available in F5 iRules, which removes the whole package from its TMM sandbox.",
            source: "Tcl man page http.n",
            examples: "package require http\nset token [http::geturl http://example.com/]\nputs [http::data $token]\nhttp::cleanup $token",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
