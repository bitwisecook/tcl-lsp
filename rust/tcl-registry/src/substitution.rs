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
//! Descriptor assistance uses
//! [`CommandRegistry::substitutions_performed`](crate::CommandRegistry::substitutions_performed).
//! Reached source evaluation uses [`crate::ResolvedInvocation::native_substitution_template`]
//! with the actual handler, available switches and retained template source.
//! Its ordered reads and errors remain separate from executable folding.
//! The selected descriptor declares option effects; the generic effect walk
//! projects authored substitution kinds independently of native object conversion.

/// The substitutions a call performs over its argument text.
///
/// Three named fields rather than a flags type, for the reason
/// [`crate::traits`] gives: a small closed set of independent facts reads
/// better as fields than as bits whose values can silently collide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

/// Actual Subst option declaration selected independently of operand objects.
/// Original enum/index caches and handler registration remain separate owners.
#[derive(Debug, Clone, Copy)]
pub struct NativeSubstitutionOptions {
    positive: bool,
    noun: &'static str,
    jim: bool,
}

/// A selected Subst option sequence has no flag interpretation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubstitutionOptionError {
    /// Positive and negative families cannot be combined.
    MixedFamilies,
    /// An ordinal does not belong to the selected declaration.
    UnknownOption,
}

impl NativeSubstitutionOptions {
    /// Select the actual supported C release or Jim option declaration.
    /// This does not select a command implementation or materialise operands.
    #[must_use]
    pub fn select(dialect: crate::InvocationDialect) -> Option<Self> {
        use tcl_dialect::{TclVersion, model::Family};
        match dialect.family()? {
            Family::Tcl => {
                let version = dialect.tcl_version?;
                Some(Self {
                    positive: version == TclVersion::V9_1,
                    jim: false,
                    noun: if version <= TclVersion::V8_5 {
                        "switch"
                    } else {
                        "option"
                    },
                })
            }
            Family::Jim
                if dialect.native_string_protocol()
                    == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084) =>
            {
                Some(Self {
                    positive: false,
                    noun: "option",
                    jim: true,
                })
            }
            _ => None,
        }
    }

    /// Static native table order, used by the original enum/index owner.
    #[must_use]
    pub const fn names(self) -> &'static [&'static str] {
        if self.positive {
            &[
                "-backslashes",
                "-commands",
                "-variables",
                "-nobackslashes",
                "-nocommands",
                "-novariables",
            ]
        } else {
            &["-nobackslashes", "-nocommands", "-novariables"]
        }
    }

    /// Native diagnostic noun for a failed option selection.
    #[must_use]
    pub const fn noun(self) -> &'static str {
        self.noun
    }

    /// Native wrong-argument usage for this declaration.
    #[must_use]
    pub const fn usage(self) -> &'static str {
        if self.positive {
            "subst ?-backslashes? ?-commands? ?-variables? ?-nobackslashes? ?-nocommands? ?-novariables? string"
        } else if self.jim {
            "subst ?options? string"
        } else {
            "subst ?-nobackslashes? ?-nocommands? ?-novariables? string"
        }
    }

    /// Fold selected declaration ordinals through the one native flag-family owner.
    ///
    /// # Errors
    /// Refuses foreign ordinals and mixed positive/negative families.
    pub fn kinds(self, selected: &[usize]) -> Result<SubstitutionKinds, SubstitutionOptionError> {
        let switches = selected
            .iter()
            .map(|&index| self.names().get(index).copied())
            .collect::<Option<Vec<_>>>()
            .ok_or(SubstitutionOptionError::UnknownOption)?;
        subst_switches(&switches)
    }
}

/// Resolver for a substituting command whose switches change which kinds run.
///
/// Takes the call's post-name arguments, exactly as
/// [`ArgRoleResolver`](crate::spec::ArgRoleResolver) does. A resolver that
/// cannot read the call — a computed switch word, an unrecognised one — must
/// answer [`SubstitutionKinds::ALL`], because assuming a substitution does not
/// happen is the answer that loses a real variable read.
pub type SubstitutionResolver = fn(args: &[&str]) -> SubstitutionKinds;

/// Native template operands selected from the actual frozen argv and option table.
/// This does not prove implementation identity or the effects of substitutions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubstitutionTemplateSelection {
    /// The native option/operand grammar rejects this invocation.
    InvalidArguments,
    /// Frozen cardinality, option values or native policies are unavailable.
    Unknown,
    /// The final value is substituted under these selected flags.
    Template {
        /// Index in the complete post-head argv.
        template_at: usize,
        /// Enabled top-level substitution operations.
        kinds: SubstitutionKinds,
        /// Whether the shared component scanner's parse rejection is native.
        parse_errors: TemplateParseErrors,
    },
}

/// Scope of the currently shared native template parser's rejection proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateParseErrors {
    /// C Tcl rejects malformed variable/bracket components incrementally.
    Rejected,
    /// Jim 0.84's independent template variable acceptance; other parser errors
    /// remain unresolved rather than becoming guest errors.
    Jim084,
}

impl TemplateParseErrors {
    /// Reached native template policy from the independently retained engine.
    #[must_use]
    pub fn for_dialect(dialect: crate::InvocationDialect) -> Option<Self> {
        match dialect.execution_point()?.family() {
            tcl_dialect::model::Family::Tcl => Some(Self::Rejected),
            tcl_dialect::model::Family::Jim
                if dialect.execution_point()?.release()
                    == tcl_dialect::model::Release::JIM_0_84 =>
            {
                Some(Self::Jim084)
            }
            _ => None,
        }
    }

    /// Variable grammar of this selected template, independent of written words.
    #[must_use]
    pub const fn variable_syntax(self) -> tcl_lexer::word_parts::TemplateVariableSyntax {
        match self {
            Self::Rejected => tcl_lexer::word_parts::TemplateVariableSyntax::CTcl,
            Self::Jim084 => tcl_lexer::word_parts::TemplateVariableSyntax::Jim084,
        }
    }
}

impl crate::ResolvedInvocation<'_, '_> {
    /// Select only an authored native template protocol. Available switches
    /// and prefix matching come from the retained invocation ingress.
    #[must_use]
    pub fn native_substitution_template(&self) -> SubstitutionTemplateSelection {
        use SubstitutionTemplateSelection as Selection;
        if self.semantics.body_execution
            != Some(crate::body_execution::BodyExecutionSpec::SubstitutionTemplate)
        {
            return Selection::Unknown;
        }
        let arguments = self.words.arguments();
        let Some(count) = arguments.exact_argv_len() else {
            return Selection::Unknown;
        };
        let Some(template_at) = count.checked_sub(1) else {
            return Selection::InvalidArguments;
        };
        let Some(dialect) = arguments.dialect() else {
            return Selection::Unknown;
        };
        let parse_errors = match dialect.family() {
            Some(tcl_dialect::model::Family::Tcl) if dialect.tcl_version.is_some() => {
                TemplateParseErrors::Rejected
            }
            Some(tcl_dialect::model::Family::Jim)
                if dialect.core_point.is_some_and(|point| {
                    point.release() == tcl_dialect::model::Release::JIM_0_84
                }) =>
            {
                TemplateParseErrors::Jim084
            }
            _ => return Selection::Unknown,
        };
        let options = self.semantics.options.available().collect::<Vec<_>>();
        let mut switches = Vec::new();
        for index in 0..template_at {
            let Some(word) = arguments.literal_at(index) else {
                return Selection::Unknown;
            };
            let Some(option) = crate::spec::resolve_available_option_prefix_with(
                &options,
                word,
                self.semantics.options.prefix_matching,
            ) else {
                return Selection::InvalidArguments;
            };
            switches.push(option.name);
        }
        let Ok(kinds) = subst_switches(&switches) else {
            return Selection::InvalidArguments;
        };
        Selection::Template {
            template_at,
            kinds,
            parse_errors,
        }
    }
}

/// World effects of a literal template under the selected native parser.
/// Operand object callbacks and handler identity are separate obligations.
/// A read, nested evaluation or unavailable parser retains the unknown envelope.
pub(crate) fn literal_template_effects(
    arguments: crate::InvocationArguments<'_>,
    template_at: usize,
    kinds: SubstitutionKinds,
) -> crate::world_effect::EffectFootprint {
    let unknown = crate::world_effect::EffectFootprint::conservative_unknown_invocation;
    let Some(dialect) = arguments.dialect() else {
        return unknown();
    };
    let Some(parser) = TemplateParseErrors::for_dialect(dialect) else {
        return unknown();
    };
    let Some(value) = arguments.literal_at(template_at) else {
        return unknown();
    };
    let flags = tcl_lexer::word_parts::SubstFlags {
        vars: kinds.variables,
        cmds: kinds.commands,
        backslashes: kinds.backslashes,
        ..tcl_lexer::word_parts::SubstFlags::default()
    };
    let Ok(parts) = tcl_lexer::word_parts::decompose_template_spanned(
        value.as_bytes(),
        flags,
        tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
        parser.variable_syntax(),
    ) else {
        return unknown();
    };
    if parts
        .iter()
        .all(|part| matches!(part.part, tcl_lexer::word_parts::WordPart::Text(_)))
    {
        crate::world_effect::EffectFootprint::default()
    } else {
        unknown()
    }
}

// The compileProc uses this same native switch table and abbreviation owner.
// Unlike runtime flags, compiler operands must already be compile-time known.
pub(crate) fn compiler_template_kinds(
    arguments: crate::InvocationArguments<'_>,
    version: tcl_dialect::TclVersion,
) -> Result<SubstitutionKinds, crate::native_compilation::NativeCompilationSelection> {
    use crate::native_compilation::NativeCompilationSelection as Selection;
    let count = arguments.exact_argv_len().ok_or(Selection::Unknown)?;
    let template_at = count.checked_sub(1).ok_or(Selection::Generic)?;
    let dialect = crate::InvocationDialect::for_version(version);
    let options = crate::commands::tcl::subst_::OPTIONS
        .iter()
        .filter(|option| option.supports_dialect(dialect.authoring_query(), None))
        .collect::<Vec<_>>();
    let mut switches = Vec::new();
    for at in 0..template_at {
        let word = arguments.literal_at(at).ok_or(Selection::Unknown)?;
        let option = crate::spec::resolve_available_option_prefix_with(
            &options,
            word,
            crate::abbrev::PrefixMatching::Enabled,
        )
        .ok_or(Selection::Generic)?;
        switches.push(option.name);
    }
    subst_switches(&switches).map_err(|_| Selection::Generic)
}

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
    subst_switches(switches).unwrap_or(SubstitutionKinds::ALL)
}

fn subst_switches(switches: &[&str]) -> Result<SubstitutionKinds, SubstitutionOptionError> {
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
            _ => return Err(SubstitutionOptionError::UnknownOption),
        }
    }
    match (saw_negated, saw_positive) {
        // The two families together are an error Tcl raises, not a shape with
        // a meaning to report.
        (true, true) => Err(SubstitutionOptionError::MixedFamilies),
        (false, true) => Ok(positive),
        _ => Ok(negated),
    }
}

#[cfg(test)]
mod tests {
    use super::{SubstitutionKinds, subst_substitutions};

    #[test]
    fn native_runtime_option_declaration_matches_registry_surface_and_family_fold() {
        // Native proof: naming.substitution.original-options-and-flag-families
        // docs/design/analysis/name-resolution-proofs/substitution-original-options-and-flag-families.md
        use super::{
            NativeSubstitutionOptions, SubstitutionOptionError,
            SubstitutionTemplateSelection as Selection,
        };
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let context = crate::model::ingress::static_context_for(engine);
            let dialect =
                crate::InvocationDialect::of_profile(context.commands().profile().unwrap());
            let options = NativeSubstitutionOptions::select(dialect).unwrap();
            assert_eq!(options.kinds(&[]), Ok(SubstitutionKinds::ALL));
            assert_eq!(
                options.kinds(&[options.names().len()]),
                Err(SubstitutionOptionError::UnknownOption)
            );
            let words = [
                crate::InvocationWord::Literal("-variables"),
                crate::InvocationWord::Literal("$x"),
            ];
            let invocation =
                crate::InvocationWords::structured(crate::InvocationWord::Literal("subst"), &words)
                    .with_dialect(dialect);
            let resolved = context
                .commands()
                .resolve_structured_invocation(invocation, dialect.authoring_query())
                .resolved()
                .unwrap();
            let declared = resolved
                .semantics
                .options
                .available()
                .map(|option| option.name)
                .collect::<std::collections::BTreeSet<_>>();
            assert_eq!(
                declared,
                options.names().iter().copied().collect(),
                "{engine}"
            );
            let selected = resolved.native_substitution_template();
            if engine == "tcl9.1" {
                assert_eq!(
                    options.names()[..3],
                    ["-backslashes", "-commands", "-variables"]
                );
                assert_eq!(
                    options.kinds(&[2]),
                    Ok(SubstitutionKinds {
                        variables: true,
                        ..SubstitutionKinds::NONE
                    })
                );
                assert_eq!(
                    options.kinds(&[2, 4]),
                    Err(SubstitutionOptionError::MixedFamilies)
                );
                assert!(matches!(
                    selected,
                    Selection::Template {
                        kinds: SubstitutionKinds {
                            backslashes: false,
                            commands: false,
                            variables: true
                        },
                        ..
                    }
                ));
            } else {
                assert_eq!(selected, Selection::InvalidArguments, "{engine}");
            }
        }
    }

    #[test]
    fn native_template_selection_retains_flags_availability_and_unknown_operands() {
        use super::SubstitutionTemplateSelection as Selection;
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let context = crate::model::ingress::static_context_for(profile);
            let dialect =
                crate::InvocationDialect::of_profile(context.commands().profile().unwrap());
            let words = [
                crate::InvocationWord::Literal("-noc"),
                crate::InvocationWord::Dynamic,
            ];
            let invocation =
                crate::InvocationWords::structured(crate::InvocationWord::Literal("subst"), &words)
                    .with_dialect(dialect);
            let resolved = context
                .commands()
                .resolve_structured_invocation(invocation, dialect.authoring_query())
                .resolved()
                .unwrap();
            assert!(
                matches!(
                    resolved.native_substitution_template(),
                    Selection::Template {
                        template_at: 1,
                        kinds: SubstitutionKinds {
                            commands: false,
                            backslashes: true,
                            variables: true
                        },
                        ..
                    }
                ),
                "{profile}"
            );
            let words = [
                crate::InvocationWord::Dynamic,
                crate::InvocationWord::Literal("$x"),
            ];
            let invocation =
                crate::InvocationWords::structured(crate::InvocationWord::Literal("subst"), &words)
                    .with_dialect(dialect);
            let resolved = context
                .commands()
                .resolve_structured_invocation(invocation, dialect.authoring_query())
                .resolved()
                .unwrap();
            assert_eq!(resolved.native_substitution_template(), Selection::Unknown);
        }
    }

    fn assert_substituted_template_declines(
        spec: &crate::native_compilation::NativeCompilationSpec,
        invocation: crate::InvocationWords<'_>,
        dialect: crate::InvocationDialect,
        entry: crate::native_compilation::NativeCompilationContext,
        profile: &str,
    ) {
        use crate::native_compilation::{
            NativeCompilationSelection as Selection, NativeCompilationWordShape as Shape,
        };
        for shape in [Shape::Substituted, Shape::BackslashLiteral] {
            assert_eq!(
                spec.select(invocation, &[Shape::Literal, shape], Some(dialect), entry),
                Selection::Generic,
                "{profile}: {shape:?}"
            );
        }
    }

    #[test]
    fn native_template_compiler_requires_original_simple_operand() {
        use crate::native_compilation::{
            NativeCompilationContext, NativeCompilationFrame, NativeCompilationMode,
            NativeCompilationSelection as Selection, NativeCompilationWordShape as Shape,
        };
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let context = crate::model::ingress::static_context_for(profile);
            let dialect =
                crate::InvocationDialect::of_profile(context.commands().profile().unwrap());
            let spec = context
                .commands()
                .get_for_surface("subst", dialect.authoring_query())
                .unwrap()
                .native_compilation
                .unwrap();
            let words = [
                crate::InvocationWord::Literal("-nocommands"),
                crate::InvocationWord::Literal("$x ${bad"),
            ];
            let invocation =
                crate::InvocationWords::structured(crate::InvocationWord::Literal("subst"), &words)
                    .with_dialect(dialect);
            let entry = NativeCompilationContext {
                mode: NativeCompilationMode::BytecodeObject,
                frame: NativeCompilationFrame::ScriptCode,
                ..Default::default()
            };
            assert_substituted_template_declines(&spec, invocation, dialect, entry, profile);
            if spec.compiler_hook_presence(dialect) == Some(true) {
                let scalar = [
                    crate::InvocationWord::Literal("-noc"),
                    crate::InvocationWord::Literal("return $x"),
                ];
                let scalar = crate::InvocationWords::structured(
                    crate::InvocationWord::Literal("subst"),
                    &scalar,
                )
                .with_dialect(dialect);
                assert!(matches!(
                    spec.select(
                        scalar,
                        &[Shape::Literal, Shape::BracedLiteral],
                        Some(dialect),
                        entry
                    ),
                    Selection::Inline { .. }
                ));
                assert_eq!(
                    spec.select(
                        scalar,
                        &[Shape::Substituted, Shape::BracedLiteral],
                        Some(dialect),
                        entry
                    ),
                    Selection::Generic
                );
                let array = [
                    crate::InvocationWord::Literal("-nocommands"),
                    crate::InvocationWord::Literal("$a([side])"),
                ];
                let array = crate::InvocationWords::structured(
                    crate::InvocationWord::Literal("subst"),
                    &array,
                )
                .with_dialect(dialect);
                assert_eq!(
                    spec.select(
                        array,
                        &[Shape::Literal, Shape::BracedLiteral],
                        Some(dialect),
                        entry
                    ),
                    Selection::Unknown
                );

                assert_eq!(
                    spec.select(
                        invocation,
                        &[Shape::Literal, Shape::BracedLiteral],
                        Some(dialect),
                        entry
                    ),
                    Selection::Unknown,
                    "accepted template compiler traversal remains separate"
                );
                assert_eq!(
                    spec.select(
                        invocation,
                        &[Shape::Literal, Shape::Opaque],
                        Some(dialect),
                        entry
                    ),
                    Selection::Unknown,
                    "missing original shape cannot prove compiler decline"
                );
            }
        }
    }

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
