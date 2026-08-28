// SPDX-License-Identifier: MIT
//! One relation mechanism, for every "X requires / conflicts with Y" fact the
//! registry knows.
//!
//! **R11** (owner ruling 2026-08-28). Before this module the registry checked
//! the same idea five times in four vocabularies: E-R14's typed
//! [`OptionRelation`] over an invocation, `ProfileSpec`'s bare `requires` and
//! `conflicts` slices walked by two hand-written loops, and the event graph's
//! `implied_profiles` / `transport` slices walked by a third. Each carried its
//! own field names, its own walker, and its own message text; none but the
//! first had lifecycle gating or evidence-carrying diagnostics.
//!
//! The core here is that first mechanism with the invocation assumption lifted
//! out of it. A [`Relation<T>`] is a [`RelationKind`] over a subject term and a
//! set of terms, judged by [`Relation::evaluate`] against any
//! [`RelationFactSource`]. The domain supplies the term type ([`OptionTerm`]
//! for an invocation, [`ProfileTerm`] for a `BIG-IP` profile stack) and a fact
//! source; everything else — the kinds, the soundness rule, the verdict, the
//! evidence behind a message — is shared.
//!
//! # Assert and infer are not the same edge
//!
//! The one thing the option domain never needed. `-command` requires
//! `-channel` **asserts**: a call supplying the first and not the second is
//! wrong, and the checker says so. `HTTP` requires `TCP` **infers**: BIG-IP
//! attaches the parent profile itself, so a configuration naming only `HTTP`
//! is not missing anything — the edge is there to *add* `TCP` to the active
//! set before anything else is judged. Same data, opposite direction, so
//! [`RelationMode`] is per edge rather than derived from the kind.
//!
//! Evaluating an [`Infer`](RelationMode::Infer) edge is a category error and
//! [`Relation::evaluate`] returns [`RelationVerdict::Satisfied`] for one
//! without looking at the facts; [`closure_over`] is what reads them.
//!
//! # Performance
//!
//! Principle P-B: the general case must not enter a VM. Every relation
//! expressible here is a few slice scans over facts the caller already
//! collected — the analyser's leading-option walk, or a virtual server's
//! attached profile list. The `constraints` hook
//! ([`crate::pack_hooks::HookFamily::Constraints`]) remains the escape hatch
//! for what this cannot phrase, and reaching for it is the exception.

use crate::dialects::DialectSet;
use crate::lifecycle::{Lifecycle, LifecycleState};

/// A term type some [`Relation`] ranges over.
///
/// One implementor per domain. The trait carries only what the shared
/// machinery needs from a term: how it reads in a diagnostic, and how an
/// author spells it in `SpecTcl`.
pub trait RelationTermKind: Copy + PartialEq + Eq + 'static {
    /// What a set of these terms is called in a generated message —
    /// `"Options"` for an invocation, `"Profiles"` for a `BIG-IP` stack.
    ///
    /// Capitalised, because it opens the sentence.
    fn collective_noun() -> &'static str;

    /// How the term reads in a diagnostic message.
    fn describe(self) -> String;

    /// The term as a `SpecTcl` author writes it — the one spelling both
    /// loaders read and the exporter writes.
    fn spelling(self) -> String;
}

/// What one subject says about a relation's terms.
///
/// Built once per judged subject and borrowed by every relation on it, so the
/// underlying scan is not repeated per relation.
pub trait RelationFactSource<T: RelationTermKind> {
    /// Whether `term` holds.
    fn holds(&self, term: T) -> TermHolds;
}

/// Whether a term holds for one subject.
///
/// `Unknown` is what keeps the checker sound: presence is always provable, but
/// *absence* is only provable when the subject was read in full. A `Requires`
/// relation over an invocation the analyser could not read to the end abstains
/// rather than accusing a call of omitting an option that a `{*}` expansion may
/// well be supplying.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TermHolds {
    /// Proven present.
    Yes,
    /// Proven absent.
    No,
    /// Not statically knowable here.
    Unknown,
}

/// What a [`Relation`] asserts about its subject and its terms.
///
/// [`Self::MutuallyExclusive`] is the whole of the relation vocabulary that
/// predates E-R14; the other three are what E-R14 adds. Each is checked
/// natively — no hook, no VM — by [`Relation::evaluate`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationKind {
    /// At most one of `terms` may hold. `subject` is unused.
    MutuallyExclusive,
    /// If `subject` holds, **every** term must hold
    /// (`bibtex::parse -command` requires `-channel`).
    Requires,
    /// If `subject` holds, **at least one** term must hold.
    RequiresOneOf,
    /// If `subject` holds, **no** term may hold — directional exclusion, for
    /// the asymmetric case a symmetric set cannot phrase
    /// (`struct::tree walk -order in` is illegal with `-type bfs`).
    Forbids,
}

impl RelationKind {
    /// The DSL statement word that authors this kind.
    #[must_use]
    pub const fn statement_word(self) -> &'static str {
        match self {
            Self::MutuallyExclusive => "option_conflict",
            Self::Requires => "option_requires",
            Self::RequiresOneOf => "option_requires_one_of",
            Self::Forbids => "option_forbids",
        }
    }

    /// Whether a violation of this kind reads as "these cannot go together"
    /// (W147) rather than "this one needs that one" (W152).
    #[must_use]
    pub const fn is_exclusion(self) -> bool {
        matches!(self, Self::MutuallyExclusive | Self::Forbids)
    }
}

/// Which direction an edge is read in — the distinction R11 had to make
/// explicit to put the profile graph and the option table on one mechanism.
///
/// See the module docs: the same `X requires Y` data means "diagnose a subject
/// that has `X` without `Y`" in one domain and "a subject with `X` also has
/// `Y`" in the other, and nothing about the kind distinguishes them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RelationMode {
    /// An unmet edge is a defect: [`Relation::evaluate`] reports it.
    #[default]
    Assert,
    /// An unmet edge is a fact to add: [`closure_over`] follows it, and
    /// [`Relation::evaluate`] ignores it.
    Infer,
}

/// A registry-declared relation between the parts of one subject.
///
/// **The declarative half of E-R14** (owner ruling 2026-08-27), generalised
/// over its term domain by R11. Every relation expressible here is evaluated
/// natively by [`Self::evaluate`] — a few slice scans over facts the caller
/// already collected — so the common case never enters the hook VM.
///
/// `dialects` is an additional availability gate; `None` inherits the owning
/// command or subcommand's dialect set. `lifecycle` gates the relation on the
/// owning package's version axis — a relation that only exists once both of
/// its operands do, which is as true of a `BIG-IP` profile pair as it is of a
/// `tcllib` option pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Relation<T: RelationTermKind> {
    /// What the relation asserts.
    pub kind: RelationKind,
    /// Whether an unmet edge is a defect or a fact to add.
    pub mode: RelationMode,
    /// The term that triggers the relation. `None` makes the relation
    /// **unconditional** — it always applies, which is what
    /// `bibtex::parse`'s "neither `-channel` nor text specified" needs — and
    /// is also how [`RelationKind::MutuallyExclusive`], which has no subject,
    /// always reads.
    pub subject: Option<T>,
    /// The terms the relation ranges over.
    pub terms: &'static [T],
    /// Tcl dialects in which this relation applies.
    pub dialects: Option<DialectSet>,
    /// Introduction / deprecation / retirement releases of this relation on
    /// the owning subject's version axis. [`Lifecycle::UNSPECIFIED`] means it
    /// applies in every version.
    pub lifecycle: Lifecycle,
    /// An author-supplied message replacing the generated one — for a library
    /// whose own error text is worth quoting.
    pub message: Option<&'static str>,
}

/// What a relation says about one subject.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelationVerdict<T: RelationTermKind> {
    /// The relation holds, or does not apply to this subject.
    Satisfied,
    /// The subject cannot be judged: something the relation reads is not
    /// statically known.
    Abstain,
    /// The relation is violated.
    Violated(RelationViolation<T>),
}

/// The evidence behind a [`RelationVerdict::Violated`], for the message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationViolation<T: RelationTermKind> {
    /// What the relation asserts.
    pub kind: RelationKind,
    /// The terms the subject actually supplied that take part in the report.
    ///
    /// Kept as terms rather than their rendered spellings because a consumer
    /// needs them to place the diagnostic: the analyser maps each back to the
    /// word that carried it.
    pub present: Vec<T>,
    /// The terms the relation wanted and did not find. Empty for an
    /// exclusion.
    pub missing: Vec<T>,
    /// The author's own message, when the relation carries one.
    pub message: Option<&'static str>,
}

impl<T: RelationTermKind> Relation<T> {
    /// A relation with no fields set — used with `..Relation::DEFAULT`.
    ///
    /// The default kind is [`RelationKind::MutuallyExclusive`] and the default
    /// mode is [`RelationMode::Assert`], so a row migrated from the retired
    /// pre-E-R14 `OptionRelation` reads exactly as it did.
    pub const DEFAULT: Self = Self {
        kind: RelationKind::MutuallyExclusive,
        mode: RelationMode::Assert,
        subject: None,
        terms: &[],
        dialects: None,
        lifecycle: Lifecycle::UNSPECIFIED,
        message: None,
    };

    /// The mutual-exclusion relation over `terms` — the shape every row
    /// authored before E-R14 had, spelled in one place so the migration is a
    /// call rather than six fields.
    #[must_use]
    pub const fn conflict(terms: &'static [T]) -> Self {
        Self {
            terms,
            ..Self::DEFAULT
        }
    }

    /// The inference edge `subject` ⇒ `terms` — `HTTP` implies `TCP`.
    ///
    /// [`RelationMode::Infer`], so [`closure_over`] follows it and
    /// [`Self::evaluate`] does not judge it.
    #[must_use]
    pub const fn implies(subject: T, terms: &'static [T]) -> Self {
        Self {
            kind: RelationKind::Requires,
            mode: RelationMode::Infer,
            subject: Some(subject),
            terms,
            ..Self::DEFAULT
        }
    }

    /// The assertion edge `subject` forbids `terms` — the directional
    /// exclusion a symmetric set cannot phrase.
    #[must_use]
    pub const fn forbids(subject: T, terms: &'static [T]) -> Self {
        Self {
            kind: RelationKind::Forbids,
            subject: Some(subject),
            terms,
            ..Self::DEFAULT
        }
    }

    /// Whether this relation applies given the resolved *`version`*.
    ///
    /// *`version`* is the guaranteed-available floor — from a `package
    /// require` for a command relation (see
    /// [`crate::version::requirement_lower_bound`]), or the resolved release
    /// for a platform one. `None` is permissive.
    #[must_use]
    pub fn available_for_version(&self, version: Option<&str>) -> bool {
        self.lifecycle.available_at(version)
    }

    /// This relation's lifecycle state at the resolved *`version`*.
    #[must_use]
    pub fn lifecycle_state(&self, version: Option<&str>) -> LifecycleState {
        self.lifecycle.state_at(version)
    }

    /// Whether this relation is active for `dialect`, inheriting the owning
    /// subject's dialect set when it has no own gate.
    #[must_use]
    pub const fn supports_dialect(
        &self,
        dialect: Option<DialectSet>,
        parent_dialects: Option<DialectSet>,
    ) -> bool {
        let Some(want) = dialect else {
            return true;
        };
        let gate = match self.dialects {
            Some(gate) => Some(gate),
            None => parent_dialects,
        };
        match gate {
            Some(have) => have.intersects(want),
            None => true,
        }
    }

    /// The relation as a `SpecTcl` author wrote it — the statement word, the
    /// subject and the terms.
    ///
    /// The one spelling shared by the loader's containment notices, the
    /// studio's row label and the export round trip, so a relation reads the
    /// same everywhere it is named.
    #[must_use]
    pub fn describe(&self) -> String {
        let terms = self
            .terms
            .iter()
            .map(|term| term.spelling())
            .collect::<Vec<_>>()
            .join(" ");
        match self.subject {
            Some(subject) => format!(
                "{} {} {{{terms}}}",
                self.kind.statement_word(),
                subject.spelling()
            ),
            None => format!("{} {{{terms}}}", self.kind.statement_word()),
        }
    }

    /// Every term this relation mentions, subject included.
    #[must_use]
    pub fn mentioned_terms(&self) -> Vec<T> {
        self.subject
            .into_iter()
            .chain(self.terms.iter().copied())
            .collect()
    }

    /// Judge one subject against this relation — **the whole declarative
    /// checker**, and the reason the common case never enters a VM.
    ///
    /// Presence is always provable, so an exclusion never needs the subject to
    /// have been read in full; absence is provable only when it was, so a
    /// `Requires` / `RequiresOneOf` over a truncated subject abstains.
    ///
    /// An [`Infer`](RelationMode::Infer) edge is not a judgement and is
    /// [`Satisfied`](RelationVerdict::Satisfied) here by construction; see
    /// [`closure_over`].
    #[must_use]
    pub fn evaluate<F: RelationFactSource<T> + ?Sized>(&self, facts: &F) -> RelationVerdict<T> {
        if self.mode == RelationMode::Infer {
            return RelationVerdict::Satisfied;
        }
        let violation = |present: Vec<T>, missing: Vec<T>| {
            RelationVerdict::Violated(RelationViolation {
                kind: self.kind,
                present,
                missing,
                message: self.message,
            })
        };
        if self.kind == RelationKind::MutuallyExclusive {
            let present: Vec<T> = self
                .terms
                .iter()
                .copied()
                .filter(|term| facts.holds(*term) == TermHolds::Yes)
                .collect();
            return if present.len() >= 2 {
                violation(present, Vec::new())
            } else {
                RelationVerdict::Satisfied
            };
        }

        // No subject means the relation is unconditional: it applies to every
        // subject, so there is nothing to prove before checking the terms.
        if let Some(subject) = self.subject {
            match facts.holds(subject) {
                TermHolds::No => return RelationVerdict::Satisfied,
                TermHolds::Unknown => return RelationVerdict::Abstain,
                TermHolds::Yes => {}
            }
        }
        let subject_terms = || self.subject.into_iter().collect::<Vec<_>>();

        match self.kind {
            RelationKind::MutuallyExclusive => unreachable!("handled above"),
            RelationKind::Requires => {
                let mut missing = Vec::new();
                for term in self.terms.iter().copied() {
                    match facts.holds(term) {
                        TermHolds::Yes => {}
                        TermHolds::No => missing.push(term),
                        TermHolds::Unknown => return RelationVerdict::Abstain,
                    }
                }
                if missing.is_empty() {
                    RelationVerdict::Satisfied
                } else {
                    violation(subject_terms(), missing)
                }
            }
            RelationKind::RequiresOneOf => {
                let mut any_unknown = false;
                for term in self.terms.iter().copied() {
                    match facts.holds(term) {
                        TermHolds::Yes => return RelationVerdict::Satisfied,
                        TermHolds::No => {}
                        TermHolds::Unknown => any_unknown = true,
                    }
                }
                if any_unknown {
                    RelationVerdict::Abstain
                } else {
                    violation(subject_terms(), self.terms.to_vec())
                }
            }
            RelationKind::Forbids => {
                let present: Vec<T> = self
                    .terms
                    .iter()
                    .copied()
                    .filter(|term| facts.holds(*term) == TermHolds::Yes)
                    .collect();
                if present.is_empty() {
                    RelationVerdict::Satisfied
                } else {
                    let mut all = subject_terms();
                    all.extend(present);
                    violation(all, Vec::new())
                }
            }
        }
    }
}

/// Follow every [`Infer`](RelationMode::Infer) edge reachable from `seed` to
/// a fixed point — the shared replacement for the hand-written transitive
/// walks the profile and event graphs each had.
///
/// `edges` is asked for the relations owned by one term; it is called once per
/// newly reached term, so a table keyed by term answers in `O(1)` and the
/// closure costs one pass over what it actually reaches rather than a scan of
/// every relation in the registry.
///
/// Only [`RelationKind::Requires`] infers: "at least one of these holds" and
/// the two exclusions say nothing about which term to add.
pub fn closure_over<T, I, F>(seed: I, mut edges: F) -> Vec<T>
where
    T: RelationTermKind,
    I: IntoIterator<Item = T>,
    F: FnMut(T) -> &'static [Relation<T>],
{
    let mut reached: Vec<T> = Vec::new();
    let mut pending: Vec<T> = Vec::new();
    for term in seed {
        if !reached.contains(&term) {
            reached.push(term);
            pending.push(term);
        }
    }
    while let Some(current) = pending.pop() {
        for relation in edges(current) {
            if relation.mode != RelationMode::Infer || relation.kind != RelationKind::Requires {
                continue;
            }
            // A subject that is not the term we are expanding does not fire:
            // the edge belongs to the owning term's table but is conditional
            // on its own subject.
            if relation.subject.is_some_and(|subject| subject != current) {
                continue;
            }
            for term in relation.terms.iter().copied() {
                if !reached.contains(&term) {
                    reached.push(term);
                    pending.push(term);
                }
            }
        }
    }
    reached
}

impl<T: RelationTermKind> RelationViolation<T> {
    /// The diagnostic message for this violation — the author's own text when
    /// the relation carries one, otherwise generated from the terms.
    ///
    /// `display_name` is the subject as written: the command (or `command
    /// subcommand`) for an invocation, the virtual server for a profile stack.
    #[must_use]
    pub fn message_for(&self, display_name: &str) -> String {
        if let Some(message) = self.message {
            return message.to_owned();
        }
        let join = |terms: &[T]| {
            terms
                .iter()
                .map(|term| term.describe())
                .collect::<Vec<_>>()
                .join(", ")
        };
        match self.kind {
            RelationKind::MutuallyExclusive => format!(
                "{} {} cannot be used together for '{display_name}'",
                T::collective_noun(),
                join(&self.present)
            ),
            RelationKind::Forbids => format!(
                "{} cannot be used together for '{display_name}'",
                join(&self.present)
            ),
            RelationKind::Requires if self.present.is_empty() => {
                format!("'{display_name}' requires {}", join(&self.missing))
            }
            RelationKind::Requires => format!(
                "{} requires {} for '{display_name}'",
                join(&self.present),
                join(&self.missing)
            ),
            RelationKind::RequiresOneOf if self.present.is_empty() => {
                format!("'{display_name}' requires one of {}", join(&self.missing))
            }
            RelationKind::RequiresOneOf => format!(
                "{} requires one of {} for '{display_name}'",
                join(&self.present),
                join(&self.missing)
            ),
        }
    }
}
