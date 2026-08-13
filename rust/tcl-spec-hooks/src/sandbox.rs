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

//! The sandbox: the closed command whitelist a hook body runs against, plus
//! the host builtins it is given instead of the ones it is denied.
//!
//! The list is `docs/design/spec-dsl-examples/README.md`, "Purity and the
//! sandbox", verbatim. It is a **whitelist**: a command the engine gains in a
//! later release is denied by default rather than becoming reachable from
//! every already-shipped pack.

use std::rc::Rc;

use tcl_engine_api::{EngineError, HostCommand, Value};

/// Every command a hook body may call, besides its family's emitter verbs and
/// the host builtins below.
///
/// Deliberately absent — and the reason the list is closed: `open`, `exec`,
/// `source`, `socket`, `after`, `interp`, `uplevel`, `upvar`, `trace`,
/// `namespace`, `proc`, `rename`, `info`, `subst`.
pub const SANDBOX_COMMANDS: &[&str] = &[
    "set", "expr", "if", "while", "for", "foreach", "switch", "return", "break", "continue",
    "incr", "lappend", "lassign", "list", "lindex", "llength", "lrange", "lreplace", "lsearch",
    "lsort", "join", "split", "string", "format", "scan", "regexp", "regsub", "dict", "binary",
];

/// `foldlist ?arg …?` — build a properly quoted Tcl list.
///
/// The conservative list builder a fold body needs: a folder whose result is a
/// list must re-quote its elements exactly as the shipped `list` fold does, or
/// the optimiser splices a value that re-parses differently. It is not merely
/// *equivalent* to the shipped folder, it **is** it
/// ([`tcl_registry::const_fold::fold_list`]), so the two cannot drift.
///
/// `list` itself is on the whitelist and does the same job inside the body;
/// `foldlist` exists for the value a body is about to `fold`, where the engine
/// would otherwise hand back a bare string.
struct FoldList;

impl HostCommand for FoldList {
    fn invoke(&self, arguments: &[Value]) -> Result<Value, EngineError> {
        let words: Vec<String> = arguments
            .iter()
            .map(|argument| argument.as_str().unwrap_or_default().to_owned())
            .collect();
        let borrowed: Vec<&str> = words.iter().map(String::as_str).collect();
        tcl_registry::const_fold::fold_list(&borrowed).map_or_else(
            || {
                Err(EngineError::Script {
                    message: "foldlist: not a foldable list".to_owned(),
                    code: None,
                })
            },
            |list| Ok(Value::string(list)),
        )
    }
}

/// The host builtins every hook body gets, whatever its family.
#[must_use]
pub fn builtins() -> Vec<(&'static str, Rc<dyn HostCommand>)> {
    vec![("foldlist", Rc::new(FoldList) as Rc<dyn HostCommand>)]
}

#[cfg(test)]
mod tests {
    use super::{SANDBOX_COMMANDS, builtins};
    use tcl_engine_api::Value;

    #[test]
    fn the_whitelist_denies_the_named_escapes() {
        for denied in [
            "open",
            "exec",
            "source",
            "socket",
            "after",
            "interp",
            "uplevel",
            "upvar",
            "trace",
            "namespace",
            "proc",
            "rename",
            "info",
            "subst",
        ] {
            assert!(
                !SANDBOX_COMMANDS.contains(&denied),
                "{denied} must not be reachable from a hook body"
            );
        }
    }

    #[test]
    fn foldlist_quotes_like_the_shipped_list_fold() {
        let [(name, foldlist)] = &builtins()[..] else {
            panic!("one builtin");
        };
        assert_eq!(*name, "foldlist");
        let folded = foldlist
            .invoke(&[Value::string("a b"), Value::string("c")])
            .expect("folds");
        assert_eq!(
            folded.as_str(),
            tcl_registry::const_fold::fold_list(&["a b", "c"]).as_deref(),
        );
    }
}
