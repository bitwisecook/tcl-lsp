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

//! Canonical argument-count arity for a proc/method parameter list.
//!
//! Tcl's argument binding is strictly positional: a call's actual words
//! bind to formals left-to-right, and a formal's default is only used
//! when the actuals run out *at that position* — it is never "skipped
//! over" to reach a later required formal.  So a required parameter
//! positioned after a defaulted one does not lower the minimum arity by
//! the defaulted ones ahead of it; it *raises* the minimum to its own
//! position, since every position up to and including it must be
//! supplied.  Confirmed against real `tclsh` 9.0.4:
//! `proc opt {a {b 5} c} {}` accepts exactly 3 arguments always — `opt 1
//! 2` fails with `wrong # args` even though only `a` and `c` lack
//! defaults, because supplying a value for `c` requires also supplying
//! one for `b`.
//!
//! A trailing parameter literally named `args` collects every remaining
//! actual argument into a list and makes the arity unbounded above; any
//! default text written on that slot (`{args ignored}`) is parsed but
//! never consulted, since the slot is never "missing" — it is simply
//! empty when there is nothing left. `args` used anywhere but last is an
//! ordinary parameter name.

use tcl_registry::Arity;

use super::types::ParamDef;

/// Compute a proc/method's declared `(min, max)` argument arity from its
/// parsed parameter list.
#[must_use]
pub fn arity_of(params: &[ParamDef]) -> Arity {
    let has_args = params.last().is_some_and(|p| p.name == "args");
    let counted = if has_args {
        &params[..params.len() - 1]
    } else {
        params
    };
    // The minimum is the 1-based position of the *last* required
    // (non-default) parameter — not a count of required parameters —
    // since every position up to it must be supplied positionally.
    let min = counted
        .iter()
        .rposition(|p| !p.has_default)
        .map_or(0, |i| i + 1);
    let min = u16::try_from(min).unwrap_or(Arity::UNLIMITED);
    let max = if has_args {
        Arity::UNLIMITED
    } else {
        u16::try_from(counted.len()).unwrap_or(Arity::UNLIMITED)
    };
    Arity::new(min, max)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(specs: &[(&str, Option<&str>)]) -> Vec<ParamDef> {
        specs
            .iter()
            .map(|(name, default)| ParamDef {
                name: (*name).to_owned(),
                has_default: default.is_some(),
                default_value: default.map(str::to_owned),
            })
            .collect()
    }

    #[test]
    fn all_required_is_exact() {
        let p = params(&[("a", None), ("b", None)]);
        assert_eq!(arity_of(&p), Arity::new(2, 2));
    }

    #[test]
    fn required_after_default_forces_exact_full_count() {
        // proc opt {a {b 5} c} — tclsh 9.0.4: `opt 1`/`opt 1 2` both
        // fail wrong-#-args; only `opt 1 2 3` succeeds; `opt 1 2 3 4`
        // fails too. min == max == 3, not 2.
        let p = params(&[("a", None), ("b", Some("5")), ("c", None)]);
        assert_eq!(arity_of(&p), Arity::new(3, 3));
    }

    #[test]
    fn interleaved_defaults_required_last_forces_full_count() {
        // proc opt2 {{a 1} b {c 2} d} — tclsh 9.0.4: only exactly 4
        // arguments are ever accepted.
        let p = params(&[("a", Some("1")), ("b", None), ("c", Some("2")), ("d", None)]);
        assert_eq!(arity_of(&p), Arity::new(4, 4));
    }

    #[test]
    fn trailing_defaults_are_genuinely_optional() {
        // proc f {a {b 5}} — the last parameter is the defaulted one,
        // so 1 or 2 arguments are both valid.
        let p = params(&[("a", None), ("b", Some("5"))]);
        assert_eq!(arity_of(&p), Arity::new(1, 2));
    }

    #[test]
    fn args_only_is_unbounded_from_zero() {
        let p = params(&[("args", None)]);
        assert_eq!(arity_of(&p), Arity::new(0, Arity::UNLIMITED));
    }

    #[test]
    fn args_not_last_is_an_ordinary_required_name() {
        // proc f {args a b} — tclsh 9.0.4: exact arity 3.
        let p = params(&[("args", None), ("a", None), ("b", None)]);
        assert_eq!(arity_of(&p), Arity::new(3, 3));
    }

    #[test]
    fn trailing_args_default_is_ignored() {
        // proc f {a {args ignored}} — tclsh 9.0.4: `f 1` succeeds with
        // args bound to the empty list, not to "ignored".
        let p = params(&[("a", None), ("args", Some("ignored"))]);
        assert_eq!(arity_of(&p), Arity::new(1, Arity::UNLIMITED));
    }

    #[test]
    fn no_params_is_exact_zero() {
        assert_eq!(arity_of(&[]), Arity::new(0, 0));
    }
}
