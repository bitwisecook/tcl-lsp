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

//! Arity, arity windows and stamp windows are self-consistent, and the gates reject each malformed shape.

use super::*;

/// Assert the basic arity invariant shared by every `Arity` (command or
/// subcommand): `min <= max`, and the unlimited sentinel agrees with `max`.
pub(super) fn assert_arity_consistent(a: Arity, what: &str) {
    assert!(
        a.min <= a.max,
        "{what}: arity min {} > max {}",
        a.min,
        a.max
    );
    assert_eq!(
        a.is_unlimited(),
        a.max == Arity::UNLIMITED,
        "{what}: is_unlimited disagrees with max sentinel"
    );
    // `accepts` must agree with the bounds at the boundaries.
    assert!(a.accepts(a.min), "{what}: arity rejects its own min");
    if !a.is_unlimited() {
        assert!(a.accepts(a.max), "{what}: arity rejects its own max");
        if a.max < u16::MAX {
            assert!(
                !a.accepts(a.max + 1),
                "{what}: arity accepts max+1 despite finite bound"
            );
        }
    }
}

/// Assert the invariants of a versioned-arity window list.
///
/// Windows are a *shipped-spec hard gate* here, matching every other lifecycle
/// (`docs/design/registry/spec-packs.md`, "Ordering and containment"): a pack degrades
/// with a notice, a compiled-in spec fails the suite.
///
/// Four properties, each a way a window set can be meaningless rather than
/// merely unusual:
///
/// 1. every window's `Arity` is internally consistent, exactly as the
///    unversioned one must be;
/// 2. every window's `Lifecycle` is ordered — an impossible window can never
///    be selected;
/// 3. each window sits inside the entity's own lifecycle: a signature shape
///    cannot be introduced before the command that has it;
/// 4. no two windows overlap, because two shapes claiming one release makes
///    selection arbitrary.
pub(super) fn assert_arity_windows_consistent(
    windows: &[ArityWindow],
    parent: Lifecycle,
    what: &str,
) {
    for (i, window) in windows.iter().enumerate() {
        assert_arity_consistent(window.arity, &format!("{what} arity window {i}"));
        assert!(
            window.lifecycle.validate().is_ok(),
            "{what}: arity window {i} has an impossible lifecycle"
        );
        if !parent.is_unspecified() && !window.lifecycle.is_unspecified() {
            let merged = window.lifecycle.intersect(parent);
            assert!(
                merged.validate().is_ok(),
                "{what}: arity window {i} narrowed by its own declaration is never available"
            );
            for (release, own, narrowed) in [
                ("introduced", window.lifecycle.introduced, merged.introduced),
                ("deprecated", window.lifecycle.deprecated, merged.deprecated),
                ("retired", window.lifecycle.retired, merged.retired),
            ] {
                if let Some(own) = own {
                    assert_eq!(
                        Some(own),
                        narrowed,
                        "{what}: arity window {i} {release} {own} reaches outside the parent"
                    );
                }
            }
        }
        for (j, other) in windows.iter().enumerate().skip(i + 1) {
            assert!(
                !window.overlaps(*other),
                "{what}: arity windows {i} and {j} both cover a release"
            );
        }
    }
}

/// The window gate above is live, not vacuous.
///
/// Every shipped spec currently declares an empty `arity_windows`, so
/// `assert_arity_windows_consistent` passes trivially across the whole sweep
/// and would keep passing if its body were deleted. This drives it directly
/// with each malformed shape it exists to reject, so the sweep's silence on
/// real specs means "nothing is wrong" rather than "nothing is checked".
#[test]
fn arity_window_gate_rejects_each_malformed_shape() {
    crate::install_surfaces();
    fn window(
        introduced: Option<&'static str>,
        retired: Option<&'static str>,
        arity: Arity,
    ) -> ArityWindow {
        ArityWindow {
            lifecycle: Lifecycle {
                introduced,
                deprecated: None,
                retired,
                deprecation_fix: None,
            },
            arity,
        }
    }
    fn rejects(windows: &[ArityWindow], parent: Lifecycle, why: &str) {
        let caught = std::panic::catch_unwind(|| {
            assert_arity_windows_consistent(windows, parent, "probe");
        });
        assert!(caught.is_err(), "the gate must reject {why}");
    }

    // A well-formed pair is accepted — the FN guard, so the cases below fail
    // for their stated reason and not because the helper rejects everything.
    assert_arity_windows_consistent(
        &[
            window(None, Some("3.0"), Arity::exact(2)),
            window(Some("3.0"), None, Arity::exact(3)),
        ],
        Lifecycle::UNSPECIFIED,
        "probe",
    );

    rejects(
        &[window(Some("3.0"), Some("2.0"), Arity::exact(2))],
        Lifecycle::UNSPECIFIED,
        "a window retired before it was introduced",
    );
    rejects(
        &[
            window(None, Some("3.0"), Arity::exact(2)),
            window(Some("2.0"), None, Arity::exact(3)),
        ],
        Lifecycle::UNSPECIFIED,
        "two windows that both cover release 2.0",
    );
    rejects(
        &[
            window(None, None, Arity::exact(1)),
            window(None, None, Arity::exact(2)),
        ],
        Lifecycle::UNSPECIFIED,
        "two unbounded windows",
    );
    rejects(
        &[window(Some("1.0"), None, Arity::exact(2))],
        Lifecycle::introduced_in("2.0"),
        "a window introduced before the command that hosts it",
    );
    rejects(
        &[window(None, None, Arity::new(5, 2))],
        Lifecycle::UNSPECIFIED,
        "a window whose own arity has min > max",
    );
}

/// Assert the invariants of a list of codegen-axis stamp windows.
///
/// A shipped-spec hard gate, like the arity windows': a pack degrades with a
/// notice, a compiled-in spec fails the suite. Two properties, each a way a
/// window set can be meaningless rather than merely unusual:
///
/// 1. every window's `Lifecycle` is ordered — an impossible window can never be
///    selected;
/// 2. no two windows overlap, because two stamps claiming one release make the
///    selection depend on declaration order.
///
/// There is no containment check against the owner's own lifecycle: a stamp
/// window is on the Tcl core's axis, and a package-owned command's lifecycle is
/// on its package's.
fn assert_stamp_windows_consistent<T: Copy>(windows: &[StampWindow<T>], what: &str) {
    for (i, window) in windows.iter().enumerate() {
        assert!(
            window.lifecycle.validate().is_ok(),
            "{what}: stamp window {i} has an impossible lifecycle"
        );
        for (j, other) in windows.iter().enumerate().skip(i + 1) {
            assert!(
                !window.overlaps(other),
                "{what}: stamp windows {i} and {j} both cover a release"
            );
        }
    }
}

/// Every stamp window list a command and its subcommands carry, through the
/// gate above.
fn assert_spec_stamp_windows_consistent(spec: &tcl_registry::CommandSpec, what: &str) {
    assert_stamp_windows_consistent(spec.codegen_hook_windows, &format!("{what} codegen_hook"));
    assert_stamp_windows_consistent(
        spec.inline_codegen_hook_windows,
        &format!("{what} inline_codegen_hook"),
    );
    assert_stamp_windows_consistent(
        spec.semantic_operation_windows,
        &format!("{what} semantic_operation"),
    );
    assert_stamp_windows_consistent(
        spec.native_lowering_windows,
        &format!("{what} native_lowering"),
    );
    for sub in spec.subcommands {
        let what = format!("{what} {}", sub.name);
        assert_stamp_windows_consistent(sub.codegen_hook_windows, &format!("{what} codegen_hook"));
        assert_stamp_windows_consistent(
            sub.inline_codegen_hook_windows,
            &format!("{what} inline_codegen_hook"),
        );
        assert_stamp_windows_consistent(
            sub.semantic_operation_windows,
            &format!("{what} semantic_operation"),
        );
    }
}

/// No shipped spec carries two stamp windows that cover one release, or one
/// that cannot be selected at any — and the gate that says so is live, not
/// vacuous.
///
/// Every shipped spec currently declares empty stamp windows, so the sweep
/// passes trivially and would keep passing if the gate's body were deleted.
/// The malformed shapes below are what it exists to reject.
#[test]
fn stamp_windows_never_overlap() {
    crate::install_surfaces();
    fn window(
        introduced: Option<&'static str>,
        retired: Option<&'static str>,
        value: u8,
    ) -> StampWindow<u8> {
        StampWindow {
            lifecycle: Lifecycle {
                introduced,
                deprecated: None,
                retired,
                deprecation_fix: None,
            },
            value,
        }
    }
    fn rejects(windows: &[StampWindow<u8>], why: &str) {
        let caught = std::panic::catch_unwind(|| {
            assert_stamp_windows_consistent(windows, "probe");
        });
        assert!(caught.is_err(), "the gate must reject {why}");
    }

    let reg = CommandRegistry::build_default();
    for name in reg.command_names() {
        for spec in reg.specs(name) {
            assert_spec_stamp_windows_consistent(spec, name);
        }
    }

    // A well-formed pair is accepted, so the cases below fail for their stated
    // reason and not because the helper rejects everything.
    assert_stamp_windows_consistent(
        &[window(None, Some("9.0"), 1), window(Some("9.0"), None, 2)],
        "probe",
    );
    rejects(
        &[window(Some("9.0"), Some("8.6"), 1)],
        "a window retired before it was introduced",
    );
    rejects(
        &[window(None, Some("9.0"), 1), window(Some("8.6"), None, 2)],
        "two windows that both cover 8.6",
    );
    rejects(
        &[window(None, None, 1), window(None, None, 2)],
        "two unbounded windows",
    );
    rejects(
        &[window(Some("9.0"), None, 1), window(Some("9.1"), None, 1)],
        "two windows of one stamp that both cover 9.1",
    );
}
