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
        // hover snippet below), never disappears — so this carries every
        // Tcl-version bit (`ALL_TCL`) rather than being version-gated. F5
        // iRules removes the whole package from its TMM sandbox (the
        // K36322151 bans); that is now modelled by this explicit `dialects`
        // group being `ALL_TCL`, alongside `open`/`socket`/`file` — which,
        // like this spec, all carry a bare `ALL_TCL` that omits the `IRULES`
        // bit and so never intersects the bare `IRULES` mask, rather than
        // relying on a disable list. iRules availability is fully explicit
        // per spec.
        dialects: Some(DialectSet::ALL_TCL),
        // Never actually invoked as a command (see the module doc
        // comment) — left unconstrained rather than modelling an
        // invocation shape that doesn't exist.
        arity: Arity::any(),
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: true,
            ..SideEffect::DEFAULT
        }],
        hover: Some(HoverSnippet {
            summary: "Client-side implementation of the HTTP/1.x protocol, built around http::geturl and a per-transaction state token.",
            synopsis: &["package require http ?version?"],
            snippet: "`::http::geturl` is the primary entry point: it runs an HTTP transaction and returns a token naming a per-transaction state array, queried through `::http::data`, `::http::code`, `::http::status`, `::http::error`, `::http::size`, and `::http::meta` (always finish with `::http::cleanup` on the token to release it). `::http::config` sets process-wide defaults such as the proxy host/port and User-Agent. `::http::register` adds custom transports such as HTTPS. Tcl 8.4's http package implements HTTP/1.0 only; HTTP/1.1 support (persistent connections via `-keepalive`, plus `-protocol`, `-method`, and `-strict` URL validation on `::http::geturl`) arrived in 8.5, and 8.6 added pipelining tunables to `::http::config` (`-pipeline`, `-postfresh`, `-repost`, `-zip`) plus `::http::quoteString` and `::http::registerError`. Tcl 9.0 added a full request/response-introspection layer — `::http::requestLine`, `::http::requestHeaders`/`::http::requestHeaderValue`, `::http::responseLine`, `::http::responseCode`, `::http::responseHeaders`/`::http::responseHeaderValue`, `::http::responseBody`, `::http::reasonPhrase`, `::http::responseInfo`, and `::http::postError` — plus a `-cookiejar` hook on `::http::config` and a `-guesstype` (default false) Content-Type-sniffing hook on `::http::geturl`. From 9.0 on, the older `::http::code`, `::http::data`, `::http::meta`, and `::http::ncode` are documented as aliases for `::http::responseLine`, `::http::responseBody`, `::http::responseHeaders`, and `::http::responseCode` respectively. Not available in F5 iRules, which removes the whole package from its TMM sandbox.",
            source: "Tcl man page http.n",
            examples: "package require http\nset token [http::geturl http://example.com/]\nputs [http::data $token]\nhttp::cleanup $token",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
