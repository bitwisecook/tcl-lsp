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

//! `oo::define` — define class members.
use crate::prelude::*;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "oo::define class defScript",
}];

/// Subcommands recognised by ``oo::define`` / ``oo::objdefine``.
/// Used to disambiguate the script-form (`oo::define Target {body}`)
/// from a subcommand call where `args[1]` is one of these words.
const OO_DEFINE_SUBCOMMANDS: &[&str] = &[
    "constructor",
    "destructor",
    "method",
    "classmethod",
    "initialise",
    "initialize",
    "private",
    "self",
    "property",
    "filter",
    "export",
    "unexport",
    "deletemethod",
    "renamemethod",
    "forward",
    "mixin",
    "superclass",
    "variable",
];

/// Resolve body argument indices for `oo::define` / `oo::objdefine`.
///
/// * `oo::define Target body` (script form, when `args[1]` is not a
///   recognised subcommand) → body at index 1.
/// * `oo::define Target constructor args body` → body at index 3.
/// * `oo::define Target destructor body` → body at index 2.
/// * `oo::define Target method name args body` → body at last index.
/// * `oo::define Target initialise body` / `initialize body` /
///   `private body` → body at index 2.
/// * `oo::define Target self constructor args body` → body at index 4.
/// * `oo::define Target self destructor body` → body at index 3.
/// * `oo::define Target self method name args body` → body at last
///   index.
/// * `oo::define Target property -set BODY ?-get BODY?` →
///   bodies after each `-set` / `-get` flag.
pub(crate) fn oo_define_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    let n = args.len();
    if n == 2 && !OO_DEFINE_SUBCOMMANDS.contains(&args[1]) {
        return vec![(1, ArgRole::Body)];
    }
    if n < 2 {
        return Vec::new();
    }
    let Ok(last) = u8::try_from(n - 1) else {
        return Vec::new();
    };
    match args[1] {
        "constructor" if n >= 4 => vec![(3, ArgRole::Body)],
        "method" | "classmethod" if n >= 5 => vec![(last, ArgRole::Body)],
        // `destructor`/`initialise`/`initialize`/`private` all take a
        // body at index 2.
        "destructor" | "initialise" | "initialize" | "private" if n >= 3 => {
            vec![(2, ArgRole::Body)]
        }
        "self" if n >= 3 => match args[2] {
            "constructor" if n >= 5 => vec![(4, ArgRole::Body)],
            "destructor" if n >= 4 => vec![(3, ArgRole::Body)],
            "method" | "classmethod" if n >= 6 => vec![(last, ArgRole::Body)],
            _ => Vec::new(),
        },
        "property" => collect_property_body_roles(args, 2),
        _ => Vec::new(),
    }
}

/// `oo::define Target property name ?-set BODY? ?-get BODY?` →
/// flag-keyed bodies. `start` is the index of the first option flag
/// (2 for `oo::define Target property`, 0 for inner `property` —
/// which folding handles separately).
pub(crate) fn collect_property_body_roles(args: &[&str], start: usize) -> Vec<(u8, ArgRole)> {
    let n = args.len();
    if n == 0 {
        return Vec::new();
    }
    args.iter()
        .enumerate()
        .skip(start)
        .take(n.saturating_sub(start + 1))
        .filter_map(|(i, &a)| {
            if (a == "-set" || a == "-get") && i + 1 < n {
                u8::try_from(i + 1).ok().map(|idx| (idx, ArgRole::Body))
            } else {
                None
            }
        })
        .collect()
}

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "oo::define",
        traits: Traits::NOT_PROC_FACTORY | Traits::LANGUAGE_KEYWORD | Traits::NEVER_INLINE_BODY,
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::at_least(2),
        arg_roles: &[(0, ArgRole::Name)],
        arg_role_resolver: Some(oo_define_arg_roles),
        return_type: Some(TclType::String),
        // Every body argument that `oo_define_arg_roles`
        // surfaces is a TclOO definition / dispatch body, never a
        // caller-frame body.  Stamping `Structural` here covers all
        // the script-bearing forms (constructor / destructor /
        // method / classmethod / initialise / initialize / private /
        // self.* / property -set / -get) plus the bare-script form
        // `oo::define Cls {body}`.
        body_kind: BodyKind::Structural,
        hover: Some(HoverSnippet {
            summary: "define and configure classes and objects",
            synopsis: &[
                "oo::define class defScript",
                "oo::define class subcommand arg ?arg ...?",
                "oo::define className ?definition?",
                "oo::define className subcommand ?arg ...?",
            ],
            snippet: "The oo::define command is used to control the configuration of classes, and the oo::objdefine command is used to control the configuration of objects (including classes as instance objects), with the configuration being applied to the entity named in the class or the object argument.",
            source: "Tcl man page define.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        definition_body: Some(&crate::definer::TCLOO_GRAMMAR),
        analyser_hook: Some(crate::hooks::AnalyserHookId::OoDefine),
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::oo_define_arg_roles;
    use crate::arg_role::ArgRole;

    // POSITIVE: every subcommand merged into the shared `n >= 3 => index 2`
    // arm (`destructor` / `initialise` / `initialize` / `private`) must
    // still resolve a body argument at index 2.
    #[test]
    fn index2_body_subcommands_resolve() {
        for sub in ["destructor", "initialise", "initialize", "private"] {
            let roles = oo_define_arg_roles(&["Target", sub, "{ body }"]);
            assert_eq!(
                roles,
                vec![(2, ArgRole::Body)],
                "{sub}: expected body at index 2"
            );
        }
    }

    // POSITIVE: arms NOT merged keep their distinct indices.
    #[test]
    fn distinct_arms_keep_their_indices() {
        assert_eq!(
            oo_define_arg_roles(&["Target", "constructor", "args", "{ body }"]),
            vec![(3, ArgRole::Body)]
        );
        assert_eq!(
            oo_define_arg_roles(&["Target", "method", "name", "args", "{ body }"]),
            vec![(4, ArgRole::Body)]
        );
        // `self destructor` is a separate (inner) arm → index 3, not 2.
        assert_eq!(
            oo_define_arg_roles(&["Target", "self", "destructor", "{ body }"]),
            vec![(3, ArgRole::Body)]
        );
    }

    // NEGATIVE: the merged arm's `n >= 3` guard must fail when the body is
    // absent — no role is surfaced.
    #[test]
    fn merged_arm_guard_rejects_too_few_args() {
        for sub in ["destructor", "initialise", "initialize", "private"] {
            // Only `Target sub` (n == 2, but `sub` is a known subcommand so
            // the bare-script fast path does not apply) → no body role.
            assert!(
                oo_define_arg_roles(&["Target", sub]).is_empty(),
                "{sub}: arity-2 form must not surface a body role"
            );
        }
    }
}
