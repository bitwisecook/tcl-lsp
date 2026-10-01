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
//! ## The capability gate
//!
//! A second gate narrows the first for a pack a *package* ships. The tier gate
//! reads where a pack was found; it cannot say how far down the dependency
//! graph the package that shipped it sits, and a pack beside a `tclpkg.tcl` is
//! a workspace-tier file whether the author wrote it or a dependency of a
//! dependency did. [`CodegenCapability::for_tier`] says what the packs of each
//! [`DependencyTier`] may declare, and [`PackCommand::dependency_tier`] says
//! which tier the command's file is at:
//!
//! - a codegen-axis stamp is refused from a tier whose capability names none
//!   ([`RefusalReason::Capability`]), in the same pass as the rules above, so
//!   a stamp must pass both gates;
//! - `alias_of` and a `runtime_backing` other than `none` are dropped, with a
//!   warning naming the tier, from a tier whose capability holds neither
//!   ([`admit_declarations`]); a `runtime_backing` that is a Tcl body is a
//!   reference body, which the compiler inlines into the code that calls the
//!   command, and is dropped from every tier but the workspace's own package.
//!
//! A command no package ships has no tier and the capability gate leaves it
//! alone. Like a refused stamp, a dropped declaration costs the command no
//! other fact.
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
use tcl_registry::model::capability::{CodegenCapability, DependencyTier, ReferenceBodies};
use tcl_registry::registry::CommandRegistry;
use tcl_registry::spec::{CommandSpec, SubCommand};
use tcl_registry::{BodySource, RuntimeBacking};

use crate::backing::BackingSyntax;
use crate::loader::PackCommand;

pub use tcl_registry::codegen_stamp::{CodegenStamp, StampSite};

/// Which part of the rule refused a stamp.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefusalReason {
    /// Rule 2: the provenance may not carry a codegen-axis stamp at all.
    TierGate,
    /// The capability gate: the package that ships the pack sits at this
    /// tier, whose capability names no codegen-axis stamp.
    Capability(DependencyTier),
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
    pub stamp: CodegenStamp,
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
            RefusalReason::Capability(tier) => {
                format!("{} may not name a codegen catalogue member", pack_of(tier))
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
        let remedy = match (self.reason, self.target) {
            (RefusalReason::Capability(_), _) => "only the workspace's own package may".to_owned(),
            (_, Some(target)) if self.alias_of == Some(target) => {
                format!("only a bundled pack may carry `alias_of {target}`'s own stamp")
            }
            (_, Some(target)) => format!("the stamp would have to sit on `alias_of {target}`"),
            (_, None) => "no shipped command carries it".to_owned(),
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

/// The subject of a capability refusal: "a direct dependency's pack".
fn pack_of(tier: DependencyTier) -> String {
    let label = tier.label();
    format!("{} {label}'s pack", indefinite_article(label))
}

/// Rule 2: whether `provenance` may carry a codegen-axis stamp at all — the
/// compiled-in catalogue and a pack bundled with the server, nothing a user
/// or a workspace authored.
#[must_use]
pub fn stamps_admitted_from(provenance: Provenance) -> bool {
    matches!(provenance, Provenance::BuiltIn | Provenance::BundledPack)
}

/// The capability gate's stamp row: whether a pack whose package sits at
/// `tier` may name a codegen-axis stamp. A pack no package ships (`None`) is
/// not narrowed by it.
#[must_use]
pub fn capability_admits_stamps(tier: Option<DependencyTier>) -> bool {
    tier.is_none_or(|tier| CodegenCapability::for_tier(tier).codegen_stamps)
}

/// Both gates: whether a codegen-axis stamp may survive from `provenance` in
/// a package at `tier`.
#[must_use]
pub fn stamps_admitted(provenance: Provenance, tier: Option<DependencyTier>) -> bool {
    stamps_admitted_from(provenance) && capability_admits_stamps(tier)
}

/// Whether `spec` carries any codegen-axis stamp, at any site.
#[must_use]
pub fn carries_stamp(spec: &CommandSpec) -> bool {
    !spec.codegen_stamps().is_empty()
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

/// What the rule refuses on `spec` at `provenance`, in a package at `tier`
/// (`None` for a pack no package ships), one [`StampRefusal`] per refused
/// stamp, without dropping anything — the preview an authoring tool reports
/// for the install it describes.
#[must_use]
pub fn stamp_refusals(
    spec: &CommandSpec,
    provenance: Provenance,
    tier: Option<DependencyTier>,
    shipped: &CommandRegistry,
) -> Vec<StampRefusal> {
    spec.codegen_stamps()
        .into_iter()
        .filter_map(|(site, stamp)| {
            refusal_reason(spec, site, stamp, provenance, tier, shipped).map(|reason| {
                StampRefusal {
                    command: spec.name,
                    site,
                    stamp,
                    provenance,
                    reason,
                    alias_of: spec.alias_of,
                    target: carrier(shipped, site, stamp),
                }
            })
        })
        .collect()
}

/// Apply the rule to one loaded command at `provenance`, in the package its
/// declaring file belongs to ([`PackCommand::dependency_tier`]): drop every
/// stamp [`stamp_refusals`] refuses and return the refusals.
///
/// A command that carries no stamp, or whose every stamp is admitted, keeps
/// its spec pointer untouched.
pub fn admit_codegen_stamps(
    command: &mut PackCommand,
    provenance: Provenance,
    shipped: &CommandRegistry,
) -> Vec<StampRefusal> {
    let refusals = stamp_refusals(command.spec, provenance, command.dependency_tier, shipped);
    if !refusals.is_empty() {
        let drops = Drops {
            stamps: refusals
                .iter()
                .map(|refusal| (refusal.site, refusal.stamp))
                .collect(),
            ..Drops::default()
        };
        command.spec = stripped(command.spec, drops);
    }
    refusals
}

/// A declaration, other than a codegen-axis stamp, that the capability gate
/// takes from a pack whose package sits too far from the workspace root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Declaration {
    /// `alias_of NAME`, which a site recorded against a shipped builtin rests
    /// on.
    AliasOf(&'static str),
    /// A `runtime_backing` other than `none` and a Tcl body, as declared.
    RuntimeBacking(RuntimeBacking),
    /// A `runtime_backing` that is a Tcl body, from a package whose tier may
    /// declare a backing: the body is a reference body, which the compiler
    /// inlines into the code of whatever calls the command, so it is held to
    /// the matrix's own row for them.
    ReferenceBody(RuntimeBacking),
}

impl Declaration {
    /// The row as the pack wrote it, for a notice.
    fn spelling(self) -> String {
        match self {
            Self::AliasOf(target) => format!("alias_of {target}"),
            // The body is the pack's own text and can run to pages.
            Self::RuntimeBacking(RuntimeBacking::TclBody {
                source: BodySource::PackText { .. },
                ..
            })
            | Self::ReferenceBody(RuntimeBacking::TclBody {
                source: BodySource::PackText { .. },
                ..
            }) => "runtime_backing tcl-body {-pack-text …}".to_owned(),
            Self::RuntimeBacking(backing) | Self::ReferenceBody(backing) => format!(
                "runtime_backing {}",
                BackingSyntax::from_backing(backing).spelling()
            ),
        }
    }

    /// What the declaration is called in the sentence that refuses it.
    const fn noun(self) -> &'static str {
        match self {
            Self::AliasOf(_) => "`alias_of`",
            Self::RuntimeBacking(_) => "a `runtime_backing`",
            Self::ReferenceBody(_) => "a reference body",
        }
    }

    /// Whether `capability` lets a pack declare it.
    const fn permitted_by(self, capability: CodegenCapability) -> bool {
        match self {
            Self::AliasOf(_) => capability.builtin_alias,
            Self::RuntimeBacking(_) => capability.runtime_backing,
            Self::ReferenceBody(_) => {
                capability.runtime_backing
                    && !matches!(capability.reference_body, ReferenceBodies::Forbidden)
            }
        }
    }

    /// Who may declare it, as the tiers the matrix names.
    const fn remedy(self) -> &'static str {
        match self {
            Self::AliasOf(_) | Self::RuntimeBacking(_) => {
                "only the workspace's own package and its direct dependencies may"
            }
            Self::ReferenceBody(_) => "only the workspace's own package may",
        }
    }
}

/// One declaration the capability gate dropped: said on the pack file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclarationRefusal {
    /// The pack command the declaration was made on.
    pub command: &'static str,
    /// What was dropped.
    pub declaration: Declaration,
    /// The tier of the package that shipped the command.
    pub tier: DependencyTier,
}

impl DeclarationRefusal {
    /// The warning the load publishes on the command's row: the declaration,
    /// the tier that may not make it, and who may.
    #[must_use]
    pub fn message(&self) -> String {
        format!(
            "`{}` refused for `{}`: {} may not declare {}; {}",
            self.declaration.spelling(),
            self.command,
            pack_of(self.tier),
            self.declaration.noun(),
            self.declaration.remedy(),
        )
    }
}

/// What the capability gate refuses on `spec` in a package at `tier`, one
/// [`DeclarationRefusal`] per declaration, without dropping anything.
///
/// A command no package ships (`tier` of `None`) is refused nothing.
#[must_use]
pub fn declaration_refusals(
    spec: &CommandSpec,
    tier: Option<DependencyTier>,
) -> Vec<DeclarationRefusal> {
    let Some(tier) = tier else {
        return Vec::new();
    };
    let capability = CodegenCapability::for_tier(tier);
    // A Tcl body is held to the reference-body row once the tier may declare a
    // backing at all; a tier that may not refuses it as the backing it is.
    let backing = match spec.runtime_backing {
        RuntimeBacking::None => None,
        backing @ RuntimeBacking::TclBody { .. } if capability.runtime_backing => {
            Some(Declaration::ReferenceBody(backing))
        }
        backing => Some(Declaration::RuntimeBacking(backing)),
    };
    let declared = spec
        .alias_of
        .map(Declaration::AliasOf)
        .into_iter()
        .chain(backing);
    declared
        .filter(|declaration| !declaration.permitted_by(capability))
        .map(|declaration| DeclarationRefusal {
            command: spec.name,
            declaration,
            tier,
        })
        .collect()
}

/// Apply the capability gate to one loaded command, in the package its
/// declaring file belongs to ([`PackCommand::dependency_tier`]): drop every
/// declaration [`declaration_refusals`] refuses and return the refusals.
///
/// A command whose declarations are all admitted keeps its spec pointer
/// untouched.
pub fn admit_declarations(command: &mut PackCommand) -> Vec<DeclarationRefusal> {
    let refusals = declaration_refusals(command.spec, command.dependency_tier);
    if !refusals.is_empty() {
        let mut drops = Drops::default();
        for refusal in &refusals {
            match refusal.declaration {
                Declaration::AliasOf(_) => drops.alias_of = true,
                Declaration::RuntimeBacking(_) | Declaration::ReferenceBody(_) => {
                    drops.runtime_backing = true;
                }
            }
        }
        command.spec = stripped(command.spec, drops);
    }
    refusals
}

/// What a strip clears from a spec: the refused stamps, and the two
/// declarations the capability gate takes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
struct Drops {
    stamps: Vec<(StampSite, CodegenStamp)>,
    alias_of: bool,
    runtime_backing: bool,
}

impl Drops {
    fn apply(&self, spec: &mut CommandSpec) {
        for &(site, stamp) in &self.stamps {
            drop_stamp(spec, site, stamp);
        }
        if self.alias_of {
            spec.alias_of = None;
        }
        if self.runtime_backing {
            spec.runtime_backing = RuntimeBacking::None;
        }
    }
}

/// `spec` with `drops` cleared, leaked once per `(spec, drops)`.
///
/// Keyed by the original's address: every spec a [`PackCommand`] holds is
/// `&'static` — leaked by the loader or compiled in — and so never freed,
/// which makes its address its identity for the life of the process. The
/// drops are part of the key because they, and nothing else, decide the
/// clone.
fn stripped(spec: &'static CommandSpec, drops: Drops) -> &'static CommandSpec {
    type Memo = FxHashMap<(usize, Drops), &'static CommandSpec>;
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
    key.1.apply(&mut clone);
    let leaked: &'static CommandSpec = Box::leak(Box::new(clone));
    memo.insert(key, leaked);
    leaked
}

/// Why `stamp` at `site` on `spec` is refused, or `None` when it is admitted.
fn refusal_reason(
    spec: &CommandSpec,
    site: StampSite,
    stamp: CodegenStamp,
    provenance: Provenance,
    tier: Option<DependencyTier>,
    shipped: &CommandRegistry,
) -> Option<RefusalReason> {
    if !stamps_admitted_from(provenance) {
        return Some(RefusalReason::TierGate);
    }
    if !capability_admits_stamps(tier)
        && let Some(tier) = tier
    {
        return Some(RefusalReason::Capability(tier));
    }
    let Some(named) = spec.alias_of else {
        return Some(RefusalReason::NoAliasOf);
    };
    let Some(target) = shipped.get(named) else {
        return Some(RefusalReason::UnknownTarget(named));
    };
    (!target.carries_codegen_stamp_at(site, stamp))
        .then_some(RefusalReason::NotTheTargetsOwn(named))
}

/// The shipped command that carries `stamp` — at the same site when one
/// does, anywhere in its spec otherwise — so a refusal can name the target
/// the stamp would have had to sit on. The first by name, so the answer does
/// not depend on the registry's map order.
fn carrier(
    shipped: &CommandRegistry,
    site: StampSite,
    stamp: CodegenStamp,
) -> Option<&'static str> {
    let mut names: Vec<&str> = shipped.command_names().collect();
    names.sort_unstable();
    let specs: Vec<&'static CommandSpec> = names
        .into_iter()
        .filter_map(|name| shipped.get_exact(name))
        .collect();
    specs
        .iter()
        .find(|spec| spec.carries_codegen_stamp_at(site, stamp))
        .or_else(|| {
            specs
                .iter()
                .find(|spec| spec.codegen_stamps().iter().any(|(_, own)| *own == stamp))
        })
        .map(|spec| spec.name)
}

/// Clear `stamp` at `site` on `spec`, leaking a fresh subcommand or form
/// slice when the stamp sits on one.
fn drop_stamp(spec: &mut CommandSpec, site: StampSite, stamp: CodegenStamp) {
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

fn clear_command(spec: &mut CommandSpec, stamp: CodegenStamp) {
    match stamp {
        CodegenStamp::Codegen(_) => spec.codegen_hook = None,
        CodegenStamp::InlineCodegen(_) => spec.inline_codegen_hook = None,
        CodegenStamp::Intrinsic(_) => spec.semantic_operation = None,
    }
}

fn clear_subcommand(sub: &mut SubCommand, stamp: CodegenStamp) {
    match stamp {
        CodegenStamp::Codegen(_) => sub.codegen_hook = None,
        CodegenStamp::InlineCodegen(_) => sub.inline_codegen_hook = None,
        CodegenStamp::Intrinsic(_) => sub.semantic_operation = None,
    }
}

fn clear_form(form: &mut CommandForm, stamp: CodegenStamp) {
    match stamp {
        CodegenStamp::Codegen(_) => form.codegen_hook = None,
        // A form carries no inline hook, so this stamp cannot sit on one.
        CodegenStamp::InlineCodegen(_) => {}
        CodegenStamp::Intrinsic(_) => form.semantic_operation = None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_registry::hooks::CodegenHookId;
    use tcl_registry::intrinsic::IntrinsicId;
    use tcl_registry::semantic_operation::SemanticOperationId;

    fn command(spec: CommandSpec) -> PackCommand {
        PackCommand {
            spec: Box::leak(Box::new(spec)),
            overrides_shipped: false,
            hooks: Vec::new(),
            clause_grammar: None,
            degraded: false,
            line: 3,
            file: std::path::PathBuf::new(),
            content_hash: 0,
            dependency_tier: None,
            reference_text: None,
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
            stamp: CodegenStamp::InlineCodegen(tcl_registry::hooks::InlineCodegenHookId::Expr),
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

    /// A command a package ships, at `tier`: `lassign`'s own stamp, `alias_of
    /// lassign`, and a shipped-builtin backing.
    fn shipped_by(tier: Option<DependencyTier>) -> PackCommand {
        let mut command = command(CommandSpec {
            name: "vendor::unpack",
            alias_of: Some("lassign"),
            runtime_backing: RuntimeBacking::shipped("lassign"),
            codegen_hook: Some(CodegenHookId::Lassign),
            ..CommandSpec::DEFAULT
        });
        command.dependency_tier = tier;
        command
    }

    /// The capability gate refuses what the provenance gate admits: a bundled
    /// pack's stamp, on the right `alias_of` target, still cannot come from a
    /// package too far from the root. The workspace's own package and a pack
    /// no package ships are not narrowed, and the notice names the tier.
    #[test]
    fn the_capability_gate_refuses_a_stamp_the_provenance_gate_admits() {
        for (tier, label) in [
            (DependencyTier::Direct, "a direct dependency's"),
            (DependencyTier::Transitive, "a transitive dependency's"),
            (DependencyTier::Development, "a development dependency's"),
        ] {
            let mut command = shipped_by(Some(tier));
            let refusals = admit_codegen_stamps(&mut command, Provenance::BundledPack, shipped());
            assert_eq!(refusals.len(), 1, "{tier:?}");
            assert_eq!(refusals[0].reason, RefusalReason::Capability(tier));
            assert_eq!(
                refusals[0].message(),
                format!(
                    "`codegen_hook Lassign` refused for `vendor::unpack`: {label} pack may not \
                     name a codegen catalogue member; only the workspace's own package may"
                )
            );
            assert_eq!(command.spec.codegen_hook, None, "{tier:?}: the stamp goes");
            assert_eq!(
                command.spec.alias_of,
                Some("lassign"),
                "{tier:?}: and only the stamp"
            );
        }
        for tier in [None, Some(DependencyTier::Root)] {
            let mut command = shipped_by(tier);
            assert!(
                admit_codegen_stamps(&mut command, Provenance::BundledPack, shipped()).is_empty(),
                "{tier:?}"
            );
            assert_eq!(command.spec.codegen_hook, Some(CodegenHookId::Lassign));
        }
    }

    /// A stamp survives only by passing both gates, and when both refuse the
    /// provenance is the reason named: it is the one the author can read off
    /// where the pack sits.
    #[test]
    fn a_stamp_must_pass_both_gates() {
        let root = Some(DependencyTier::Root);
        let direct = Some(DependencyTier::Direct);
        for (provenance, tier, admitted) in [
            (Provenance::BundledPack, None, true),
            (Provenance::BundledPack, root, true),
            (Provenance::BundledPack, direct, false),
            (Provenance::WorkspaceTrusted, None, false),
            (Provenance::WorkspaceTrusted, root, false),
            (Provenance::WorkspaceTrusted, direct, false),
        ] {
            assert_eq!(
                stamps_admitted(provenance, tier),
                admitted,
                "{provenance:?} in {tier:?}"
            );
        }
        let spec = shipped_by(direct).spec;
        let refusals = stamp_refusals(spec, Provenance::WorkspaceTrusted, direct, shipped());
        assert_eq!(refusals.len(), 1);
        assert_eq!(refusals[0].reason, RefusalReason::TierGate);
    }

    /// The matrix decides `alias_of` and a backing: a direct dependency
    /// keeps both, a transitive or development one neither, and a pack no
    /// package ships is not narrowed.
    #[test]
    fn the_matrix_decides_which_declarations_a_command_may_keep() {
        let spec = shipped_by(None).spec;
        let refused = |tier| -> Vec<Declaration> {
            declaration_refusals(spec, tier)
                .into_iter()
                .map(|refusal| refusal.declaration)
                .collect()
        };
        let both = vec![
            Declaration::AliasOf("lassign"),
            Declaration::RuntimeBacking(RuntimeBacking::shipped("lassign")),
        ];
        assert!(refused(None).is_empty());
        assert!(refused(Some(DependencyTier::Root)).is_empty());
        assert!(refused(Some(DependencyTier::Direct)).is_empty());
        assert_eq!(refused(Some(DependencyTier::Transitive)), both);
        assert_eq!(refused(Some(DependencyTier::Development)), both);

        // A command that declares neither has nothing to refuse anywhere.
        let bare = CommandSpec {
            name: "vendor::bare",
            ..CommandSpec::DEFAULT
        };
        assert!(declaration_refusals(&bare, Some(DependencyTier::Transitive)).is_empty());
    }

    /// The remedy the messages give ("only the workspace's own package and its
    /// direct dependencies may") is the matrix's own answer for both
    /// declarations, so a change to the matrix cannot leave the text behind.
    #[test]
    fn the_remedy_names_the_tiers_the_matrix_permits() {
        for tier in [
            DependencyTier::Root,
            DependencyTier::Direct,
            DependencyTier::Transitive,
            DependencyTier::Development,
        ] {
            let capability = CodegenCapability::for_tier(tier);
            let named = matches!(tier, DependencyTier::Root | DependencyTier::Direct);
            assert_eq!(Declaration::AliasOf("x").permitted_by(capability), named);
            assert_eq!(
                Declaration::RuntimeBacking(RuntimeBacking::HostNative).permitted_by(capability),
                named
            );
            // A reference body is the workspace's own package's alone.
            assert_eq!(
                Declaration::ReferenceBody(tcl_body()).permitted_by(capability),
                tier == DependencyTier::Root
            );
        }
    }

    fn tcl_body() -> RuntimeBacking {
        RuntimeBacking::pack_text("proc vendor::double {x} {expr {$x * 2}}")
    }

    /// A Tcl body is a reference body, which only the workspace's own package
    /// may supply: a direct dependency keeps every other backing and loses
    /// this one, said in the reference body's own words, and a further tier,
    /// which may declare no backing at all, loses it as a backing.
    #[test]
    fn only_the_workspaces_own_package_may_supply_a_reference_body() {
        let body = || {
            let mut command = command(CommandSpec {
                name: "vendor::double",
                runtime_backing: tcl_body(),
                ..CommandSpec::DEFAULT
            });
            command.dependency_tier = None;
            command
        };
        let mut shipped_by_no_package = body();
        assert!(admit_declarations(&mut shipped_by_no_package).is_empty());
        assert_eq!(shipped_by_no_package.spec.runtime_backing, tcl_body());

        let mut root = body();
        root.dependency_tier = Some(DependencyTier::Root);
        assert!(admit_declarations(&mut root).is_empty());
        assert_eq!(root.spec.runtime_backing, tcl_body());

        let mut direct = body();
        direct.dependency_tier = Some(DependencyTier::Direct);
        let refusals = admit_declarations(&mut direct);
        assert_eq!(
            refusals
                .iter()
                .map(DeclarationRefusal::message)
                .collect::<Vec<_>>(),
            vec![
                "`runtime_backing tcl-body {-pack-text …}` refused for `vendor::double`: a \
                 direct dependency's pack may not declare a reference body; only the \
                 workspace's own package may"
            ]
        );
        assert_eq!(direct.spec.runtime_backing, RuntimeBacking::None);

        let mut transitive = body();
        transitive.dependency_tier = Some(DependencyTier::Transitive);
        let refusals = admit_declarations(&mut transitive);
        assert_eq!(refusals.len(), 1);
        assert_eq!(
            refusals[0].declaration,
            Declaration::RuntimeBacking(tcl_body()),
            "a tier with no backings refuses it as one"
        );
        assert_eq!(transitive.spec.runtime_backing, RuntimeBacking::None);

        // A backing that is no Tcl body is not a reference body.
        let mut host = command(CommandSpec {
            name: "vendor::native",
            runtime_backing: RuntimeBacking::HostNative,
            ..CommandSpec::DEFAULT
        });
        host.dependency_tier = Some(DependencyTier::Direct);
        assert!(admit_declarations(&mut host).is_empty());
    }

    #[test]
    fn a_refusal_names_the_declaration_the_command_and_the_tier() {
        let mut command = shipped_by(Some(DependencyTier::Transitive));
        let refusals = admit_declarations(&mut command);
        let messages: Vec<String> = refusals.iter().map(DeclarationRefusal::message).collect();
        assert_eq!(
            messages,
            vec![
                "`alias_of lassign` refused for `vendor::unpack`: a transitive dependency's pack \
                 may not declare `alias_of`; only the workspace's own package and its direct \
                 dependencies may",
                "`runtime_backing shipped-builtin lassign` refused for `vendor::unpack`: a \
                 transitive dependency's pack may not declare a `runtime_backing`; only the \
                 workspace's own package and its direct dependencies may",
            ]
        );

        // A body carried in the pack is named, not quoted: it can run to pages.
        let text = Declaration::RuntimeBacking(RuntimeBacking::pack_text("return 1"));
        assert_eq!(text.spelling(), "runtime_backing tcl-body {-pack-text …}");
    }

    /// A dropped declaration costs the command nothing else, and a command
    /// whose declarations are all admitted keeps its spec pointer.
    #[test]
    fn a_dropped_declaration_costs_no_other_fact() {
        let mut direct = shipped_by(Some(DependencyTier::Direct));
        let before = direct.spec;
        assert!(admit_declarations(&mut direct).is_empty());
        assert!(
            std::ptr::eq(direct.spec, before),
            "nothing dropped, nothing cloned"
        );

        let mut far = shipped_by(Some(DependencyTier::Development));
        let original = far.spec;
        assert_eq!(admit_declarations(&mut far).len(), 2);
        assert!(!std::ptr::eq(far.spec, original));
        assert_eq!(far.spec.alias_of, None);
        assert_eq!(far.spec.runtime_backing, RuntimeBacking::None);
        let without = CommandSpec {
            alias_of: None,
            runtime_backing: RuntimeBacking::None,
            ..original.clone()
        };
        assert_eq!(
            format!("{:?}", far.spec),
            format!("{without:?}"),
            "the stamp and every other fact are as loaded"
        );
    }

    /// The stripped spec is leaked once per original and drop set: a reload
    /// of an unchanged pack reuses it, whichever gate drops.
    #[test]
    fn a_repeated_declaration_refusal_reuses_its_stripped_spec() {
        let original: &'static CommandSpec = Box::leak(Box::new(CommandSpec {
            name: "vendor::unpack",
            alias_of: Some("lassign"),
            ..CommandSpec::DEFAULT
        }));
        let strip = || {
            let mut loaded = command(CommandSpec::DEFAULT);
            loaded.spec = original;
            loaded.dependency_tier = Some(DependencyTier::Transitive);
            admit_declarations(&mut loaded);
            loaded.spec
        };
        let first = strip();
        assert!(!std::ptr::eq(first, original));
        assert!(
            std::ptr::eq(first, strip()),
            "the second reload leaks nothing"
        );
        assert_eq!(first.alias_of, None);
    }
}
