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

//! Descriptive argument counts over the shared selected formal grammar.
//!
//! Original source consumers retain `SignatureSourceFormalParameters` and
//! project its `argument_count_shape` through `arity_from_count_shape`. They
//! never reconstruct count from reporting parameter names. Declaration records
//! retain `SourceFormalCount`; incremental headers copy only its body-free
//! `SourceFormalCountProjection`, which cannot recreate source or binding rights.
//!
//! C Tcl and Jim use different count grammars. C Tcl binds positions in order:
//! a required formal after a default requires every preceding position, and
//! only final `args` is variadic. Jim counts required formals independently and
//! permits its rest formal among other parameters. `arity_of_in` is explicit
//! authored metadata under a selected grammar; `arity_of` is its C Tcl form.
//! Neither function establishes an original formal producer or entered frame.
//! The finite native matrix in `procedure-original-formal-count-shape.md` covers seven
//! ASCII definitions and caught argument counts zero through five. Larger
//! registry count limits are representation constraints, not native acceptance.

use tcl_registry::Arity;

use super::types::ParamDef;

/// Whether a parameter list ends in the formal literally named `args`,
/// which collects every argument past the others into one list.
#[must_use]
pub fn is_variadic(params: &[ParamDef]) -> bool {
    params.last().is_some_and(|p| p.name == "args")
}

/// Compute a proc/method's declared `(min, max)` argument arity from its
/// parsed parameter list.
#[must_use]
pub fn arity_of(params: &[ParamDef]) -> Arity {
    arity_of_in(params, tcl_dialect::ParameterGrammar::Tcl)
}

/// Descriptive count shape under an independently selected formal grammar.
/// Decoded names/defaults share the same owner as native argument binding.
#[must_use]
pub fn arity_of_in(params: &[ParamDef], grammar: tcl_dialect::ParameterGrammar) -> Arity {
    let shape = tcl_syntax::formal_params::formal_argument_count_shape(
        params
            .iter()
            .map(|parameter| (parameter.name == "args", parameter.has_default)),
        grammar,
    );
    arity_from_count_shape(shape).unwrap_or_else(Arity::any)
}

/// Checked projection of the shared count owner. A finite count cannot borrow
/// the registry's unbounded sentinel; unrepresentable bounds remain unknown.
#[must_use]
pub fn arity_from_count_shape(
    shape: tcl_syntax::formal_params::FormalArgumentCountShape,
) -> Option<Arity> {
    let min = u16::try_from(shape.minimum).ok()?;
    if min == Arity::UNLIMITED {
        return None;
    }
    let max = match shape.maximum {
        Some(maximum) => {
            let max = u16::try_from(maximum).ok()?;
            if max == Arity::UNLIMITED {
                return None;
            }
            max
        }
        None => Arity::UNLIMITED,
    };
    Some(Arity::new(min, max))
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
    fn finite_source_count_bounds_cannot_borrow_unlimited_sentinel() {
        // naming.procedure.original-formal-count-shape
        // docs/design/analysis/name-resolution-proofs/procedure-original-formal-count-shape.md
        // Registry metadata representability only; the native proof observes
        // argv counts 0..5 and does not establish these large-count boundaries.
        use tcl_syntax::formal_params::FormalArgumentCountShape as Shape;
        let sentinel = usize::from(Arity::UNLIMITED);
        for shape in [
            Shape {
                minimum: sentinel,
                maximum: None,
            },
            Shape {
                minimum: 0,
                maximum: Some(sentinel),
            },
            Shape {
                minimum: sentinel + 1,
                maximum: None,
            },
            Shape {
                minimum: 0,
                maximum: Some(sentinel + 1),
            },
        ] {
            assert!(arity_from_count_shape(shape).is_none());
        }
        assert_eq!(
            arity_from_count_shape(Shape {
                minimum: sentinel - 1,
                maximum: Some(sentinel - 1)
            }),
            Some(Arity::exact(Arity::UNLIMITED - 1))
        );
        assert_eq!(
            arity_from_count_shape(Shape {
                minimum: 1,
                maximum: None
            }),
            Some(Arity::at_least(1))
        );
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
