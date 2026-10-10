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

//! **Codegen-axis stamps** — the spec fields that change emitted code rather
//! than what the editor knows: [`CommandSpec::codegen_hook`],
//! [`CommandSpec::inline_codegen_hook`], and an
//! [`Intrinsic`](SemanticOperationId::Intrinsic) `semantic_operation`, on a
//! command, one of its subcommands, or one of its invocation forms.
//!
//! One question has one answer here: does a spec carry *this* stamp at
//! *this* site? Two consumers ask it. The loader's stamp rejection rule
//! (`tcl_spectcl::stamps`) admits a pack's stamp only when the command's
//! [`CommandSpec::alias_of`] target carries the same stamp at the same site;
//! codegen ([`ResolvedCall::stamp_identity`]) records that target's identity
//! at a specialised site only on the same condition, so a pack command gains
//! a builtin's identity exactly where the stamp it specialises on is that
//! builtin's own (`docs/design/compiler/registry-consumer-contracts.md`
//! § *The loader's stamp rejection rule*).

use crate::forms::CommandForm;
use crate::hooks::{CodegenHookId, InlineCodegenHookId};
use crate::intrinsic::IntrinsicId;
use crate::registry::{CommandRegistry, ResolvedCall};
use crate::semantic_operation::SemanticOperationId;
use crate::spec::{CommandSpec, SubCommand};
use crate::stamp_window::{StampWindow, stamps};

/// One codegen-axis stamp, as the row that states it names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CodegenStamp {
    /// `codegen_hook -native ID`.
    Codegen(CodegenHookId),
    /// `inline_codegen_hook -native ID`.
    InlineCodegen(InlineCodegenHookId),
    /// `semantic_operation {Intrinsic ID}`.
    Intrinsic(IntrinsicId),
}

impl CodegenStamp {
    /// The stamp as its row reads, without the `-native` flag.
    #[must_use]
    pub fn spelling(self) -> String {
        match self {
            Self::Codegen(id) => format!("codegen_hook {id:?}"),
            Self::InlineCodegen(id) => format!("inline_codegen_hook {id:?}"),
            Self::Intrinsic(id) => format!("semantic_operation {{Intrinsic {id:?}}}"),
        }
    }
}

/// Where on a command a stamp sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StampSite {
    /// The command itself.
    Command,
    /// The subcommand of this name.
    Subcommand(&'static str),
    /// The invocation form of this name.
    Form(&'static str),
}

fn intrinsic(operation: SemanticOperationId) -> Option<CodegenStamp> {
    match operation {
        SemanticOperationId::Intrinsic(id) => Some(CodegenStamp::Intrinsic(id)),
        _ => None,
    }
}

/// The stamps one command or subcommand level carries: the unversioned fields
/// and every window's value, so a windowed stamp is held to the same rule as an
/// unversioned one and cannot be a way round it.
fn level_stamps<'a>(
    codegen: (Option<CodegenHookId>, &'a [StampWindow<CodegenHookId>]),
    inline_codegen: (
        Option<InlineCodegenHookId>,
        &'a [StampWindow<InlineCodegenHookId>],
    ),
    semantic: (
        Option<SemanticOperationId>,
        &'a [StampWindow<SemanticOperationId>],
    ),
) -> impl Iterator<Item = CodegenStamp> + 'a {
    stamps(codegen.0, codegen.1)
        .map(CodegenStamp::Codegen)
        .chain(stamps(inline_codegen.0, inline_codegen.1).map(CodegenStamp::InlineCodegen))
        .chain(stamps(semantic.0, semantic.1).filter_map(intrinsic))
}

fn command_level(spec: &CommandSpec) -> impl Iterator<Item = CodegenStamp> + '_ {
    level_stamps(
        (spec.codegen_hook, spec.codegen_hook_windows),
        (spec.inline_codegen_hook, spec.inline_codegen_hook_windows),
        (spec.semantic_operation, spec.semantic_operation_windows),
    )
}

fn subcommand_level(sub: &SubCommand) -> impl Iterator<Item = CodegenStamp> + '_ {
    level_stamps(
        (sub.codegen_hook, sub.codegen_hook_windows),
        (sub.inline_codegen_hook, sub.inline_codegen_hook_windows),
        (sub.semantic_operation, sub.semantic_operation_windows),
    )
}

fn form_level(form: &CommandForm) -> impl Iterator<Item = CodegenStamp> {
    [
        form.codegen_hook.map(CodegenStamp::Codegen),
        form.semantic_operation.and_then(intrinsic),
    ]
    .into_iter()
    .flatten()
}

impl CommandSpec {
    /// Every codegen-axis stamp this spec carries, with its site, in
    /// declaration order: the command's own, then each subcommand's, then
    /// each invocation form's.
    #[must_use]
    pub fn codegen_stamps(&self) -> Vec<(StampSite, CodegenStamp)> {
        let mut out: Vec<(StampSite, CodegenStamp)> = command_level(self)
            .map(|stamp| (StampSite::Command, stamp))
            .collect();
        for sub in self.subcommands {
            out.extend(subcommand_level(sub).map(|stamp| (StampSite::Subcommand(sub.name), stamp)));
        }
        for form in self.command_forms {
            out.extend(form_level(form).map(|stamp| (StampSite::Form(form.name), stamp)));
        }
        // A stamp a level carries in two windows, or beside its unversioned
        // field, is one stamp at one site.
        let mut seen = Vec::with_capacity(out.len());
        out.retain(|entry| {
            let fresh = !seen.contains(entry);
            if fresh {
                seen.push(*entry);
            }
            fresh
        });
        out
    }

    /// Whether this spec carries `stamp` at `site`: on itself, on its
    /// subcommand of that name, or on its form of that name.
    #[must_use]
    pub fn carries_codegen_stamp_at(&self, site: StampSite, stamp: CodegenStamp) -> bool {
        match site {
            StampSite::Command => command_level(self).any(|own| own == stamp),
            StampSite::Subcommand(name) => self
                .subcommands
                .iter()
                .filter(|sub| sub.name == name)
                .any(|sub| subcommand_level(sub).any(|own| own == stamp)),
            StampSite::Form(name) => self
                .command_forms
                .iter()
                .filter(|form| form.name == name)
                .any(|form| form_level(form).any(|own| own == stamp)),
        }
    }
}

impl ResolvedCall<'_> {
    /// The site `stamp` answers from in this call — the matched form's when
    /// it carries the stamp, else the matched subcommand's, else the
    /// command's: the precedence [`CommandRegistry::resolve_call`] gives the
    /// effective hooks.
    #[must_use]
    pub fn stamp_site(&self, stamp: CodegenStamp) -> StampSite {
        if let Some(form) = self.form
            && form_level(form).any(|own| own == stamp)
        {
            return StampSite::Form(form.name);
        }
        if let Some(sub) = self.sub
            && subcommand_level(sub).any(|own| own == stamp)
        {
            return StampSite::Subcommand(sub.name);
        }
        StampSite::Command
    }

    /// The registry identity a site specialised on `stamp` records for this
    /// call: the command's [`CommandSpec::alias_of`] target when `registry`'s
    /// spec for that target carries the same stamp at the same site — the
    /// one shape the stamp rejection rule admits on a pack command — and the
    /// resolved spec's own name otherwise.
    ///
    /// The condition is what keeps the identity honest: an `-override` of a
    /// shipped command keeps the shipped command's codegen hook through the
    /// security floor whatever `alias_of` it declares, and recording that
    /// unrelated target would let a runtime alias of the builtin's name to
    /// the target admit the builtin's specialised code for a different
    /// command.
    #[must_use]
    pub fn stamp_identity(&self, registry: &CommandRegistry, stamp: CodegenStamp) -> &'static str {
        let site = self.stamp_site(stamp);
        self.spec
            .alias_of
            .and_then(|target| registry.get(target))
            .filter(|target| target.carries_codegen_stamp_at(site, stamp))
            .map_or(self.spec.name, |target| target.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry_with(spec: CommandSpec) -> CommandRegistry {
        let mut registry = CommandRegistry::build_default();
        registry.insert(spec);
        registry
    }

    /// A pack command whose stamp is its `alias_of` target's own records the
    /// target; the same command naming a target that lacks the stamp, or
    /// naming none, records its own name.
    #[test]
    fn the_identity_is_the_target_only_where_the_stamp_is_the_target_s_own() {
        let stamp = CodegenStamp::Codegen(CodegenHookId::Lassign);
        for (alias_of, identity) in [
            (Some("lassign"), "lassign"),
            (Some("lsort"), "vendor::unpack"),
            (None, "vendor::unpack"),
        ] {
            let registry = registry_with(CommandSpec {
                name: "vendor::unpack",
                alias_of,
                codegen_hook: Some(CodegenHookId::Lassign),
                ..CommandSpec::DEFAULT
            });
            let call = registry
                .resolve_call("vendor::unpack", &["$l", "a"], None)
                .expect("the pack command resolves");
            assert_eq!(call.stamp_site(stamp), StampSite::Command);
            assert_eq!(
                call.stamp_identity(&registry, stamp),
                identity,
                "{alias_of:?}"
            );
        }
    }

    /// The overriding shape the condition exists for: a command that keeps a
    /// shipped hook under its own name, naming an unrelated `alias_of`,
    /// records its own name, never the unrelated target.
    #[test]
    fn an_override_with_an_unrelated_alias_keeps_its_own_identity() {
        let mut shipped = CommandRegistry::build_default()
            .get("lassign")
            .expect("lassign ships")
            .clone();
        shipped.alias_of = Some("lsort");
        let registry = registry_with(shipped);
        let call = registry
            .resolve_call("lassign", &["$l", "a"], None)
            .expect("lassign resolves");
        let stamp = CodegenStamp::Codegen(call.codegen_hook.expect("lassign's own hook"));
        assert_eq!(call.stamp_identity(&registry, stamp), "lassign");
    }

    /// A stamp in a window is a stamp: a spec carries it, lists it once however
    /// many windows or fields hold it, and a rule that reads the list sees it.
    #[test]
    fn a_windowed_stamp_is_carried_and_listed_once() {
        use crate::lifecycle::Lifecycle;
        use crate::semantic_operation::SemanticOperationId;
        use crate::stamp_window::StampWindow;

        const LASSIGN: &[StampWindow<CodegenHookId>] = &[
            StampWindow {
                lifecycle: Lifecycle::UNSPECIFIED.retired_from("9.0"),
                value: CodegenHookId::Lassign,
            },
            StampWindow {
                lifecycle: Lifecycle::introduced_in("9.0"),
                value: CodegenHookId::Lassign,
            },
        ];
        const LENGTH: &[StampWindow<SemanticOperationId>] = &[StampWindow {
            lifecycle: Lifecycle::introduced_in("9.0"),
            value: SemanticOperationId::Intrinsic(IntrinsicId::StringLength),
        }];
        let stamp = CodegenStamp::Codegen(CodegenHookId::Lassign);
        let length = CodegenStamp::Intrinsic(IntrinsicId::StringLength);

        let windowed_only = CommandSpec {
            name: "vendor::unpack",
            codegen_hook_windows: LASSIGN,
            semantic_operation_windows: LENGTH,
            ..CommandSpec::DEFAULT
        };
        assert!(windowed_only.carries_codegen_stamp_at(StampSite::Command, stamp));
        assert!(windowed_only.carries_codegen_stamp_at(StampSite::Command, length));
        assert_eq!(
            windowed_only.codegen_stamps(),
            [(StampSite::Command, stamp), (StampSite::Command, length)],
            "two windows holding one hook are one stamp"
        );

        let beside = CommandSpec {
            codegen_hook: Some(CodegenHookId::Lassign),
            ..windowed_only
        };
        assert_eq!(
            beside.codegen_stamps(),
            [(StampSite::Command, stamp), (StampSite::Command, length)],
            "and so is the plain field beside them"
        );

        let sub = SubCommand {
            name: "length",
            semantic_operation_windows: LENGTH,
            ..SubCommand::DEFAULT
        };
        let ensemble = CommandSpec {
            name: "vendor::str",
            subcommands: Box::leak(Box::new([sub])),
            ..CommandSpec::DEFAULT
        };
        assert!(ensemble.carries_codegen_stamp_at(StampSite::Subcommand("length"), length));
        assert!(!ensemble.carries_codegen_stamp_at(StampSite::Command, length));
        assert_eq!(
            ensemble.codegen_stamps(),
            [(StampSite::Subcommand("length"), length)]
        );
    }

    /// A call that took its hook from a window records the `alias_of` target's
    /// identity where the target carries the same stamp, as a plain one does.
    #[test]
    fn a_windowed_stamp_records_the_target_s_identity_where_it_is_the_target_s_own() {
        use crate::lifecycle::Lifecycle;
        use crate::stamp_window::StampWindow;
        use tcl_dialect::model::{Family, SurfaceQuery};

        const FROM_9: &[StampWindow<CodegenHookId>] = &[StampWindow {
            lifecycle: Lifecycle::introduced_in("9.0"),
            value: CodegenHookId::Lassign,
        }];
        let registry = registry_with(CommandSpec {
            name: "vendor::unpack",
            alias_of: Some("lassign"),
            codegen_hook_windows: FROM_9,
            ..CommandSpec::DEFAULT
        });
        let stamp = CodegenStamp::Codegen(CodegenHookId::Lassign);
        let at_9 = registry
            .resolve_call(
                "vendor::unpack",
                &["$l", "a"],
                Some(SurfaceQuery::core(Family::Tcl, "9.0")),
            )
            .expect("the pack command resolves");
        assert_eq!(at_9.codegen_hook, Some(CodegenHookId::Lassign));
        assert_eq!(at_9.stamp_site(stamp), StampSite::Command);
        assert_eq!(at_9.stamp_identity(&registry, stamp), "lassign");
        let at_8 = registry
            .resolve_call(
                "vendor::unpack",
                &["$l", "a"],
                Some(SurfaceQuery::core(Family::Tcl, "8.6")),
            )
            .expect("the pack command resolves");
        assert_eq!(at_8.codegen_hook, None, "no window covers 8.6");
    }

    /// Sites are matched by name: `string length`'s intrinsic is carried at
    /// `Subcommand("length")` and nowhere else.
    #[test]
    fn a_subcommand_stamp_is_carried_at_its_own_site() {
        let registry = CommandRegistry::build_default();
        let string = registry.get("string").expect("string ships");
        let length = CodegenStamp::Intrinsic(IntrinsicId::StringLength);
        assert!(string.carries_codegen_stamp_at(StampSite::Subcommand("length"), length));
        assert!(!string.carries_codegen_stamp_at(StampSite::Subcommand("range"), length));
        assert!(!string.carries_codegen_stamp_at(StampSite::Command, length));
        assert!(
            string
                .codegen_stamps()
                .contains(&(StampSite::Subcommand("length"), length))
        );
        assert_eq!(
            length.spelling(),
            "semantic_operation {Intrinsic StringLength}"
        );
    }
}
