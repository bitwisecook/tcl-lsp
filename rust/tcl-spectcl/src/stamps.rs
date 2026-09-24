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

//! The loader's **stamp rejection rule** —
//! `docs/design/compiler/registry-consumer-contracts.md` § *The loader's
//! stamp rejection rule*.
//!
//! A *codegen-axis stamp* is a pack row that changes emitted code rather
//! than what the editor knows: `codegen_hook`, `inline_codegen_hook`, or
//! `semantic_operation {Intrinsic …}`, on a command, one of its
//! subcommands, or one of its invocation forms. A pack may claim one only as
//! the shipped builtin's own, and only from a tier that ships with the
//! server:
//!
//! 1. **The stamp must be the target's own.** It is admitted only when the
//!    command declares `alias_of NAME` — never by a name match, and never by
//!    an alias the realm infers from script statements — and the shipped
//!    command `NAME` carries that same stamp at the same site (the command
//!    itself, the subcommand of the same name, the form of the same name).
//! 2. **A tier gate decides who may stamp at all.** Only
//!    [`Provenance::BuiltIn`] and [`Provenance::BundledPack`] may; every
//!    other provenance is refused with its label named.
//! 3. **A refusal drops the stamp and nothing else.** The command keeps
//!    every analysis fact it declared — arity, roles, effects, hooks — so a
//!    refused stamp never costs its author an analysis fact (the authority
//!    ruling), and the refusal names the provenance and the target the stamp
//!    would have had to sit on.
//!
//! [`crate::pack::load_sources`] applies the rule to every merged command,
//! and the load publishes each refusal as a warning on the command's row;
//! [`crate::install`] asserts that no stamp reaches a registry from a
//! provenance the gate refuses.
//!
//! ## What a refusal costs
//!
//! A pack command's spec is leaked (`&'static`), so dropping a stamp clones
//! the spec, clears the field, and leaks the clone. The clone is memoised on
//! the original spec and the exact stamps dropped, and the loader's snapshot
//! cache hands an unchanged pack's original specs back on every reload, so a
//! reload — or a Spec Studio preview asked again — reuses the one clone
//! rather than leaking another. What leaks is one clone per distinct
//! evaluated spec that carries a refused stamp: bounded by pack edits, the
//! same order as what the loader itself leaks for each edit of a pack. (A
//! pack the snapshot cache does not hold — a target-dependent one, or one
//! with `include` rows — is re-evaluated, and its specs re-leaked, on every
//! load; its clones follow that count.)

use std::sync::{Mutex, OnceLock, PoisonError};

use rustc_hash::FxHashMap;
use tcl_dialect::model::Provenance;
use tcl_registry::forms::CommandForm;
use tcl_registry::hooks::{CodegenHookId, InlineCodegenHookId};
use tcl_registry::intrinsic::IntrinsicId;
use tcl_registry::registry::CommandRegistry;
use tcl_registry::semantic_operation::SemanticOperationId;
use tcl_registry::spec::{CommandSpec, SubCommand};

use crate::loader::PackCommand;

/// One codegen-axis stamp, as the row that states it names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stamp {
    /// `codegen_hook -native ID`.
    Codegen(CodegenHookId),
    /// `inline_codegen_hook -native ID`.
    InlineCodegen(InlineCodegenHookId),
    /// `semantic_operation {Intrinsic ID}`.
    Intrinsic(IntrinsicId),
}

impl Stamp {
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

/// Where on a pack command a stamp sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StampSite {
    /// The command itself.
    Command,
    /// The subcommand of this name.
    Subcommand(&'static str),
    /// The invocation form of this name.
    Form(&'static str),
}

/// Which part of the rule refused a stamp.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefusalReason {
    /// Rule 2: the provenance may not carry a codegen-axis stamp at all.
    TierGate,
    /// Rule 1: the command declares no `alias_of`, so nothing names the
    /// builtin whose stamp this would be.
    NoAliasOf,
    /// Rule 1: `alias_of` names no shipped command.
    UnknownTarget(&'static str),
    /// Rule 1: `alias_of` names a shipped command that does not carry this
    /// stamp at this site.
    NotTheTargetsOwn(&'static str),
}

/// One refused stamp: dropped from the command, reported on the pack file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StampRefusal {
    /// The pack command the stamp was declared on.
    pub command: &'static str,
    /// Where on the command.
    pub site: StampSite,
    /// The stamp that was dropped.
    pub stamp: Stamp,
    /// The provenance the pack loaded under.
    pub provenance: Provenance,
    /// Why it was dropped.
    pub reason: RefusalReason,
    /// The command's own `alias_of`, as declared.
    pub alias_of: Option<&'static str>,
    /// The shipped command that carries this stamp — the `alias_of` target
    /// the stamp would have had to sit on — or `None` when no shipped command
    /// carries it.
    pub target: Option<&'static str>,
}

impl StampRefusal {
    /// The warning the load publishes on the command's row: the stamp, where
    /// it sat, why it was refused (the provenance named, for the tier gate),
    /// and the target it would have had to sit on.
    #[must_use]
    pub fn message(&self) -> String {
        let why = match self.reason {
            RefusalReason::TierGate => {
                let label = tcl_registry::model::provenance_label(self.provenance);
                format!(
                    "{} {label} pack may not name a codegen catalogue member",
                    indefinite_article(label)
                )
            }
            RefusalReason::NoAliasOf => "a codegen-axis stamp must be a shipped builtin's own, \
                 and the command declares no `alias_of`"
                .to_owned(),
            RefusalReason::UnknownTarget(named) => {
                format!("`alias_of {named}` names no shipped command")
            }
            RefusalReason::NotTheTargetsOwn(named) => {
                format!("`alias_of {named}` names a shipped command that does not carry it")
            }
        };
        let remedy = match self.target {
            Some(target) if self.alias_of == Some(target) => {
                format!("only a bundled pack may carry `alias_of {target}`'s own stamp")
            }
            Some(target) => format!("the stamp would have to sit on `alias_of {target}`"),
            None => "no shipped command carries it".to_owned(),
        };
        format!(
            "`{}` refused for {}: {why}; {remedy}",
            self.stamp.spelling(),
            self.site_spelling()
        )
    }

    fn site_spelling(&self) -> String {
        match self.site {
            StampSite::Command => format!("`{}`", self.command),
            StampSite::Subcommand(name) => format!("`{} {name}`", self.command),
            StampSite::Form(name) => format!("`{}` (form `{name}`)", self.command),
        }
    }
}

/// "a" or "an" before a provenance label.
fn indefinite_article(label: &str) -> &'static str {
    if label.starts_with("un") || label.starts_with(['a', 'e', 'i', 'o']) {
        "an"
    } else {
        "a"
    }
}

/// Rule 2: whether `provenance` may carry a codegen-axis stamp at all — the
/// compiled-in catalogue and a pack bundled with the server, nothing a user
/// or a workspace authored.
#[must_use]
pub fn stamps_admitted_from(provenance: Provenance) -> bool {
    matches!(provenance, Provenance::BuiltIn | Provenance::BundledPack)
}

/// Whether `spec` carries any codegen-axis stamp, at any site.
#[must_use]
pub fn carries_stamp(spec: &CommandSpec) -> bool {
    !stamps_of(spec).is_empty()
}

/// The shipped registry the rule reads a target from: the permissive
/// all-Tcl view of the compiled catalogue, the one the E-R2 gate already
/// treats as "a compiled command". A load is dialect-free, and the binding
/// identity a stamp claims is the builtin's name in every release that has
/// it; a release without the target simply never binds to it at run time.
#[must_use]
pub fn shipped() -> &'static CommandRegistry {
    crate::environment::lenient_store()
}

/// What the rule refuses on `spec` at `provenance`, one [`StampRefusal`]
/// per refused stamp, without dropping anything — the preview an authoring
/// tool reports for the install it describes.
#[must_use]
pub fn stamp_refusals(
    spec: &CommandSpec,
    provenance: Provenance,
    shipped: &CommandRegistry,
) -> Vec<StampRefusal> {
    stamps_of(spec)
        .into_iter()
        .filter_map(|(site, stamp)| {
            refusal_reason(spec, site, stamp, provenance, shipped).map(|reason| StampRefusal {
                command: spec.name,
                site,
                stamp,
                provenance,
                reason,
                alias_of: spec.alias_of,
                target: carrier(shipped, site, stamp),
            })
        })
        .collect()
}

/// Apply the rule to one loaded command at `provenance`: drop every stamp
/// [`stamp_refusals`] refuses and return the refusals.
///
/// A command that carries no stamp, or whose every stamp is admitted, keeps
/// its spec pointer untouched.
pub fn admit_codegen_stamps(
    command: &mut PackCommand,
    provenance: Provenance,
    shipped: &CommandRegistry,
) -> Vec<StampRefusal> {
    let refusals = stamp_refusals(command.spec, provenance, shipped);
    if !refusals.is_empty() {
        let drops = refusals
            .iter()
            .map(|refusal| (refusal.site, refusal.stamp))
            .collect();
        command.spec = stripped(command.spec, drops);
    }
    refusals
}

/// `spec` with `drops` cleared, leaked once per `(spec, drops)`.
///
/// Keyed by the original's address: every spec a [`PackCommand`] holds is
/// `&'static` — leaked by the loader or compiled in — and so never freed,
/// which makes its address its identity for the life of the process. The
/// drops are part of the key because they, and nothing else, decide the
/// clone.
fn stripped(spec: &'static CommandSpec, drops: Vec<(StampSite, Stamp)>) -> &'static CommandSpec {
    type Memo = FxHashMap<(usize, Vec<(StampSite, Stamp)>), &'static CommandSpec>;
    static STRIPPED: OnceLock<Mutex<Memo>> = OnceLock::new();
    let key = (std::ptr::from_ref(spec).addr(), drops);
    let mut memo = STRIPPED
        .get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    if let Some(done) = memo.get(&key) {
        return done;
    }
    let mut clone = spec.clone();
    for &(site, stamp) in &key.1 {
        drop_stamp(&mut clone, site, stamp);
    }
    let leaked: &'static CommandSpec = Box::leak(Box::new(clone));
    memo.insert(key, leaked);
    leaked
}

/// Why `stamp` at `site` on `spec` is refused, or `None` when it is admitted.
fn refusal_reason(
    spec: &CommandSpec,
    site: StampSite,
    stamp: Stamp,
    provenance: Provenance,
    shipped: &CommandRegistry,
) -> Option<RefusalReason> {
    if !stamps_admitted_from(provenance) {
        return Some(RefusalReason::TierGate);
    }
    let Some(named) = spec.alias_of else {
        return Some(RefusalReason::NoAliasOf);
    };
    let Some(target) = shipped.get(named) else {
        return Some(RefusalReason::UnknownTarget(named));
    };
    (!carried_at(target, site, stamp)).then_some(RefusalReason::NotTheTargetsOwn(named))
}

/// Every stamp `spec` carries, with its site, in declaration order: the
/// command's own, then each subcommand's, then each form's.
fn stamps_of(spec: &CommandSpec) -> Vec<(StampSite, Stamp)> {
    let mut out: Vec<(StampSite, Stamp)> = command_stamps(spec)
        .map(|stamp| (StampSite::Command, stamp))
        .collect();
    for sub in spec.subcommands {
        out.extend(subcommand_stamps(sub).map(|stamp| (StampSite::Subcommand(sub.name), stamp)));
    }
    for form in spec.command_forms {
        out.extend(form_stamps(form).map(|stamp| (StampSite::Form(form.name), stamp)));
    }
    out
}

fn intrinsic(operation: Option<SemanticOperationId>) -> Option<Stamp> {
    match operation {
        Some(SemanticOperationId::Intrinsic(id)) => Some(Stamp::Intrinsic(id)),
        _ => None,
    }
}

fn command_stamps(spec: &CommandSpec) -> impl Iterator<Item = Stamp> {
    [
        spec.codegen_hook.map(Stamp::Codegen),
        spec.inline_codegen_hook.map(Stamp::InlineCodegen),
        intrinsic(spec.semantic_operation),
    ]
    .into_iter()
    .flatten()
}

fn subcommand_stamps(sub: &SubCommand) -> impl Iterator<Item = Stamp> {
    [
        sub.codegen_hook.map(Stamp::Codegen),
        sub.inline_codegen_hook.map(Stamp::InlineCodegen),
        intrinsic(sub.semantic_operation),
    ]
    .into_iter()
    .flatten()
}

fn form_stamps(form: &CommandForm) -> impl Iterator<Item = Stamp> {
    [
        form.codegen_hook.map(Stamp::Codegen),
        intrinsic(form.semantic_operation),
    ]
    .into_iter()
    .flatten()
}

/// Whether `target` carries `stamp` at `site`: on itself, on its subcommand
/// of the same name, or on its form of the same name.
fn carried_at(target: &CommandSpec, site: StampSite, stamp: Stamp) -> bool {
    match site {
        StampSite::Command => command_stamps(target).any(|own| own == stamp),
        StampSite::Subcommand(name) => target
            .subcommands
            .iter()
            .filter(|sub| sub.name == name)
            .any(|sub| subcommand_stamps(sub).any(|own| own == stamp)),
        StampSite::Form(name) => target
            .command_forms
            .iter()
            .filter(|form| form.name == name)
            .any(|form| form_stamps(form).any(|own| own == stamp)),
    }
}

/// The shipped command that carries `stamp` — at the same site when one
/// does, anywhere in its spec otherwise — so a refusal can name the target
/// the stamp would have had to sit on. The first by name, so the answer does
/// not depend on the registry's map order.
fn carrier(shipped: &CommandRegistry, site: StampSite, stamp: Stamp) -> Option<&'static str> {
    let mut names: Vec<&str> = shipped.command_names().collect();
    names.sort_unstable();
    let specs: Vec<&'static CommandSpec> = names
        .into_iter()
        .filter_map(|name| shipped.get_exact(name))
        .collect();
    specs
        .iter()
        .find(|spec| carried_at(spec, site, stamp))
        .or_else(|| {
            specs
                .iter()
                .find(|spec| stamps_of(spec).iter().any(|(_, own)| *own == stamp))
        })
        .map(|spec| spec.name)
}

/// Clear `stamp` at `site` on `spec`, leaking a fresh subcommand or form
/// slice when the stamp sits on one.
fn drop_stamp(spec: &mut CommandSpec, site: StampSite, stamp: Stamp) {
    match site {
        StampSite::Command => clear_command(spec, stamp),
        StampSite::Subcommand(name) => {
            let mut subs = spec.subcommands.to_vec();
            for sub in subs.iter_mut().filter(|sub| sub.name == name) {
                clear_subcommand(sub, stamp);
            }
            spec.subcommands = Box::leak(subs.into_boxed_slice());
        }
        StampSite::Form(name) => {
            let mut forms = spec.command_forms.to_vec();
            for form in forms.iter_mut().filter(|form| form.name == name) {
                clear_form(form, stamp);
            }
            spec.command_forms = Box::leak(forms.into_boxed_slice());
        }
    }
}

fn clear_command(spec: &mut CommandSpec, stamp: Stamp) {
    match stamp {
        Stamp::Codegen(_) => spec.codegen_hook = None,
        Stamp::InlineCodegen(_) => spec.inline_codegen_hook = None,
        Stamp::Intrinsic(_) => spec.semantic_operation = None,
    }
}

fn clear_subcommand(sub: &mut SubCommand, stamp: Stamp) {
    match stamp {
        Stamp::Codegen(_) => sub.codegen_hook = None,
        Stamp::InlineCodegen(_) => sub.inline_codegen_hook = None,
        Stamp::Intrinsic(_) => sub.semantic_operation = None,
    }
}

fn clear_form(form: &mut CommandForm, stamp: Stamp) {
    match stamp {
        Stamp::Codegen(_) => form.codegen_hook = None,
        // A form carries no inline hook, so this stamp cannot sit on one.
        Stamp::InlineCodegen(_) => {}
        Stamp::Intrinsic(_) => form.semantic_operation = None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command(spec: CommandSpec) -> PackCommand {
        PackCommand {
            spec: Box::leak(Box::new(spec)),
            overrides_shipped: false,
            hooks: Vec::new(),
            clause_grammar: None,
            degraded: false,
            line: 3,
            file: std::path::PathBuf::new(),
        }
    }

    /// A pack `string` whose `length` subcommand carries the shipped
    /// `string length`'s own intrinsic: admitted from a bundled pack only
    /// through `alias_of string`, and the refusal names the subcommand site.
    #[test]
    fn a_subcommand_stamp_is_the_same_named_subcommand_s_own() {
        let length = SubCommand {
            name: "length",
            semantic_operation: Some(SemanticOperationId::Intrinsic(IntrinsicId::StringLength)),
            ..SubCommand::DEFAULT
        };
        let spec = |alias_of| CommandSpec {
            name: "vendor::str",
            alias_of,
            subcommands: Box::leak(Box::new([length.clone()])),
            ..CommandSpec::DEFAULT
        };

        let mut admitted = command(spec(Some("string")));
        let before = admitted.spec;
        assert!(admit_codegen_stamps(&mut admitted, Provenance::BundledPack, shipped()).is_empty());
        assert!(
            std::ptr::eq(admitted.spec, before),
            "an admitted stamp keeps its spec"
        );

        let mut refused = command(spec(None));
        let refusals = admit_codegen_stamps(&mut refused, Provenance::BundledPack, shipped());
        assert_eq!(refusals.len(), 1);
        assert_eq!(refusals[0].site, StampSite::Subcommand("length"));
        assert_eq!(refusals[0].target, Some("string"));
        assert_eq!(
            refusals[0].message(),
            "`semantic_operation {Intrinsic StringLength}` refused for `vendor::str length`: a \
             codegen-axis stamp must be a shipped builtin's own, and the command declares no \
             `alias_of`; the stamp would have to sit on `alias_of string`"
        );
        assert_eq!(refused.spec.subcommands[0].semantic_operation, None);
        assert_eq!(
            refused.spec.subcommands[0].name, "length",
            "only the stamp goes"
        );
    }

    /// A form stamp is checked against the target's form of the same name,
    /// and an `alias_of` naming nothing shipped is its own refusal.
    #[test]
    fn a_form_stamp_and_an_unknown_target() {
        let form = CommandForm {
            name: "pair",
            codegen_hook: Some(CodegenHookId::Lassign),
            ..CommandForm::DEFAULT
        };
        let mut unknown = command(CommandSpec {
            name: "vendor::unpack",
            alias_of: Some("vendor::no_such_builtin"),
            command_forms: Box::leak(Box::new([form])),
            ..CommandSpec::DEFAULT
        });
        let refusals = admit_codegen_stamps(&mut unknown, Provenance::BundledPack, shipped());
        assert_eq!(refusals.len(), 1);
        assert_eq!(refusals[0].site, StampSite::Form("pair"));
        assert_eq!(
            refusals[0].reason,
            RefusalReason::UnknownTarget("vendor::no_such_builtin")
        );
        assert!(
            refusals[0]
                .message()
                .starts_with("`codegen_hook Lassign` refused for `vendor::unpack` (form `pair`): "),
            "{}",
            refusals[0].message()
        );
        assert_eq!(unknown.spec.command_forms[0].codegen_hook, None);
        assert!(!carries_stamp(unknown.spec));
    }

    /// Rule 2 is decided by the provenance alone, and the refusal of a stamp
    /// no shipped command carries says so rather than inventing a target.
    #[test]
    fn only_the_compiled_catalogue_and_a_bundled_pack_may_stamp() {
        for provenance in [
            Provenance::User,
            Provenance::WorkspaceTrusted,
            Provenance::WorkspaceUntrusted,
            Provenance::StudioOverride,
            Provenance::Document,
        ] {
            assert!(!stamps_admitted_from(provenance), "{provenance:?}");
        }
        assert!(stamps_admitted_from(Provenance::BuiltIn));
        assert!(stamps_admitted_from(Provenance::BundledPack));

        let refusal = StampRefusal {
            command: "vendor::x",
            site: StampSite::Command,
            stamp: Stamp::InlineCodegen(InlineCodegenHookId::Expr),
            provenance: Provenance::Document,
            reason: RefusalReason::TierGate,
            alias_of: None,
            target: None,
        };
        assert_eq!(
            refusal.message(),
            "`inline_codegen_hook Expr` refused for `vendor::x`: a document pack may not name a \
             codegen catalogue member; no shipped command carries it"
        );
    }

    /// The stripped spec is leaked once per original and drop set, so a
    /// reload of an unchanged pack reuses it.
    #[test]
    fn a_repeated_refusal_reuses_its_stripped_spec() {
        let original: &'static CommandSpec = Box::leak(Box::new(CommandSpec {
            name: "vendor::unpack",
            codegen_hook: Some(CodegenHookId::Lassign),
            ..CommandSpec::DEFAULT
        }));
        let strip = || {
            let mut loaded = command(CommandSpec::DEFAULT);
            loaded.spec = original;
            admit_codegen_stamps(&mut loaded, Provenance::WorkspaceTrusted, shipped());
            loaded.spec
        };
        let first = strip();
        assert!(!std::ptr::eq(first, original));
        assert!(
            std::ptr::eq(first, strip()),
            "the second reload leaks nothing"
        );
        assert_eq!(first.codegen_hook, None);
    }
}
