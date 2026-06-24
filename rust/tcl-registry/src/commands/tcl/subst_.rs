//! `subst` — perform Tcl substitutions on a string.
use crate::prelude::*;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-nobackslashes",
        takes_value: false,
        value_hint: "",
        detail: "",
        dialects: None,
    },
    OptionSpec {
        name: "-nocommands",
        takes_value: false,
        value_hint: "",
        detail: "",
        dialects: None,
    },
    OptionSpec {
        name: "-novariables",
        takes_value: false,
        value_hint: "",
        detail: "",
        dialects: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "subst ?options? string",
}];

/// Fold a literal `subst string`.
///
/// `subst` performs variable, command, and backslash substitution on
/// its string argument — even inside braces (`subst {$x}` substitutes
/// `$x`).  The Rust O129 path hands this the *raw* literal argument (it
/// has no upstream `$var` resolution — that is the deferred B2 work), so
/// to stay sound we fold only the bare `subst string` form whose string
/// carries **no** substitution: no `$`, `[`, or `\`.  Such a string is
/// its own `subst` result.  Anything with a substitution bails
/// (a stricter subset of what a fold with upstream resolution +
/// the `[command]` caller-bail could decide).  The option forms
/// (`-nobackslashes` / …) change which substitutions apply, so the
/// multi-arg form bails too.
fn fold_subst(args: &[&str]) -> Option<String> {
    let [s] = args else {
        return None;
    };
    if s.contains(['$', '[', '\\']) {
        return None;
    }
    Some((*s).to_owned())
}

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "subst",
        traits: Traits::TAINT_SINK | Traits::IS_UNESCAPE | Traits::PERFORMS_SUBSTITUTION,
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        const_fold: Some(fold_subst),
        hover: Some(HoverSnippet {
            summary: "Perform backslash, command, and variable substitutions.",
            synopsis: &[
                "subst ?options? string",
                "subst ?-nobackslashes? ?-nocommands? ?-novariables? string",
            ],
            snippet: "**Security**: Without `-nocommands`, any `[cmd]` in the string is executed as Tcl. Use `-nocommands` when only variable substitution is needed: `subst -nocommands $template`. For safe templating, prefer `[string map]` or `[format]`.",
            source: "Tcl subst(1)",
            examples: "",
            return_value: "The string with substitutions applied.",
        }),
        forms: FORMS,
        options: OPTIONS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::fold_subst;

    #[test]
    fn subst_folds_only_substitution_free_strings() {
        // A plain `subst string` is its own
        // result; anything with $ / [ / \ bails (sound subset).
        assert_eq!(fold_subst(&["hello"]).as_deref(), Some("hello"));
        assert_eq!(fold_subst(&["a b c"]).as_deref(), Some("a b c"));
        assert_eq!(fold_subst(&["$x"]), None, "variable substitution bails");
        assert_eq!(fold_subst(&["[cmd]"]), None, "command substitution bails");
        assert_eq!(fold_subst(&["a\\nb"]), None, "backslash bails");
        assert_eq!(
            fold_subst(&["-novariables", "x"]),
            None,
            "option form bails"
        );
    }
}
