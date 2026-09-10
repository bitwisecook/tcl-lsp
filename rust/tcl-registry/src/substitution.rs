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
//! Consumers, all through
//! [`CommandRegistry::substitutions_performed`](crate::CommandRegistry::substitutions_performed):
//! the analyser's W102 `subst`-injection check, and the two compile-time
//! template folders — `Lowerer::eval_subst_nocommands_body` and
//! `specialise_factories`'s template extraction — which fold only the call
//! whose answer is the effect set their evaluator reproduces.

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

/// Resolver for a substituting command whose switches change which kinds run.
///
/// Takes the call's post-name arguments, exactly as
/// [`ArgRoleResolver`](crate::spec::ArgRoleResolver) does. A resolver that
/// cannot read the call — a computed switch word, an unrecognised one — must
/// answer [`SubstitutionKinds::ALL`], because assuming a substitution does not
/// happen is the answer that loses a real variable read.
pub type SubstitutionResolver = fn(args: &[&str]) -> SubstitutionKinds;

/// The `subst` switch grammar, as the manpages describe it.
///
/// Two families. The negated one (`-nobackslashes`, `-nocommands`,
/// `-novariables`; every release) starts from all three and turns the named
/// ones off. The positive one (`-backslashes`, `-commands`, `-variables`; Tcl
/// 9.1) starts from none and turns the named ones on. The manpage forbids
/// combining the families, so a call that does is not a call this can read.
///
/// The final argument is the string operand, never a switch. Anything else
/// that is not a recognised switch — a computed word, an abbreviation, a
/// misspelling — makes the call unreadable, and unreadable answers `ALL`.
#[must_use]
pub fn subst_substitutions(args: &[&str]) -> SubstitutionKinds {
    let Some((_operand, switches)) = args.split_last() else {
        return SubstitutionKinds::ALL;
    };
    let mut negated = SubstitutionKinds::ALL;
    let mut positive = SubstitutionKinds::NONE;
    let mut saw_negated = false;
    let mut saw_positive = false;
    for switch in switches {
        match *switch {
            "-nobackslashes" => {
                negated.backslashes = false;
                saw_negated = true;
            }
            "-nocommands" => {
                negated.commands = false;
                saw_negated = true;
            }
            "-novariables" => {
                negated.variables = false;
                saw_negated = true;
            }
            "-backslashes" => {
                positive.backslashes = true;
                saw_positive = true;
            }
            "-commands" => {
                positive.commands = true;
                saw_positive = true;
            }
            "-variables" => {
                positive.variables = true;
                saw_positive = true;
            }
            _ => return SubstitutionKinds::ALL,
        }
    }
    match (saw_negated, saw_positive) {
        // The two families together are an error Tcl raises, not a shape with
        // a meaning to report.
        (true, true) => SubstitutionKinds::ALL,
        (false, true) => positive,
        _ => negated,
    }
}

#[cfg(test)]
mod tests {
    use super::{SubstitutionKinds, subst_substitutions};

    #[test]
    fn tp_no_switches_runs_every_substitution() {
        assert_eq!(
            subst_substitutions(&["hello $name"]),
            SubstitutionKinds::ALL
        );
    }

    #[test]
    fn tp_a_negated_switch_turns_off_only_its_own_kind() {
        assert_eq!(
            subst_substitutions(&["-novariables", "hello $name"]),
            SubstitutionKinds {
                variables: false,
                ..SubstitutionKinds::ALL
            }
        );
        assert_eq!(
            subst_substitutions(&["-nocommands", "hello $name"]),
            SubstitutionKinds {
                commands: false,
                ..SubstitutionKinds::ALL
            }
        );
    }

    /// The Tcl 9.1 positive family names the *only* kinds that run.
    #[test]
    fn tp_a_positive_switch_turns_on_only_its_own_kind() {
        assert_eq!(
            subst_substitutions(&["-variables", "hello $name"]),
            SubstitutionKinds {
                variables: true,
                ..SubstitutionKinds::NONE
            }
        );
        assert_eq!(
            subst_substitutions(&["-backslashes", "-commands", "x"]),
            SubstitutionKinds {
                variables: false,
                ..SubstitutionKinds::ALL
            }
        );
    }

    /// A call this cannot read answers `ALL`, because assuming a substitution
    /// does not happen is the answer that loses a real variable read.
    #[test]
    fn fp_an_unreadable_call_answers_every_substitution() {
        // Mixed families: an error, not a shape.
        assert_eq!(
            subst_substitutions(&["-novariables", "-commands", "x"]),
            SubstitutionKinds::ALL
        );
        // A computed switch word.
        assert_eq!(subst_substitutions(&["$opt", "x"]), SubstitutionKinds::ALL);
        // An abbreviation the spec does not declare.
        assert_eq!(
            subst_substitutions(&["-novar", "x"]),
            SubstitutionKinds::ALL
        );
        // No operand at all.
        assert_eq!(subst_substitutions(&[]), SubstitutionKinds::ALL);
    }
}
