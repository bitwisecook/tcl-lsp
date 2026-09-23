//! Which Tcl substitutions a call performs over its own argument text.
//!
//! [`Traits::PERFORMS_SUBSTITUTION`](crate::Traits::PERFORMS_SUBSTITUTION) says
//! *that* a command substitutes; this says *which kinds*, for the call in hand.
//! `subst` runs all three by default and its switches turn them off (or, from
//! Tcl 9.1, name the only ones to turn on), so "does this argument read a
//! variable?" is a per-call question that a consumer cannot answer from the
//! trait alone — and must not answer by matching option spellings itself.
//!
//! The three kinds are the ones Tcl's own substitution phase distinguishes,
//! and they are independent: `subst -novariables {a$b[c]}` leaves `$b` as
//! literal text while still evaluating `[c]`, whose *contents* are ordinary
//! script and substitute normally.
//!
//! Which kinds a call performs is not computed here: `subst`'s switches are
//! option rows declaring their [`crate::option_effect::OptionEffect`] on the
//! substitution axis, in two families, and the answer is the projection of the
//! generic option-effect walk ([`crate::option_effect::substitution_kinds`],
//! [`crate::CommandSpec::substitutions_performed`]).
//!
//! Consumers, all through
//! [`CommandRegistry::substitutions_performed`](crate::CommandRegistry::substitutions_performed):
//! the analyser's W102 `subst`-injection check, the two compile-time template
//! folders — `Lowerer::eval_subst_nocommands_body` and `specialise_factories`'s
//! template extraction — which fold only the call whose answer is the effect
//! set their evaluator reproduces, and extract-proc's literal cut.

/// The substitutions a call performs over its argument text.
///
/// Three named fields rather than a flags type, for the reason
/// [`crate::traits`] gives: a small closed set of independent facts reads
/// better as fields than as bits whose values can silently collide.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubstitutionKinds {
    /// `\n` and friends become their escape values.
    pub backslashes: bool,
    /// `[…]` is evaluated as a script.
    pub commands: bool,
    /// `$name` is replaced by the variable's value.
    pub variables: bool,
}

impl SubstitutionKinds {
    /// Every kind — what a command performs when nothing narrows it, and the
    /// conservative answer whenever a call's switches cannot be read.
    pub const ALL: Self = Self {
        backslashes: true,
        commands: true,
        variables: true,
    };

    /// No kind at all — the argument is literal text.
    pub const NONE: Self = Self {
        backslashes: false,
        commands: false,
        variables: false,
    };
}
