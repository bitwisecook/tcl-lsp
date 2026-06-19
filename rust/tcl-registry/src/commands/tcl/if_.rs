//! `if` — conditional execution with optional elseif/else clauses.

use crate::prelude::*;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "if expr1 ?then? body1 ?elseif expr2 ?then? body2 ...? ?else? ?bodyN?",
}];

/// Dynamic arg role resolver for `if`/`elseif`/`else` chains.
///
/// Walks the argument list recognising `then`, `elseif`, `else`
/// keywords and classifying each positional argument as either
/// `Expr` (conditions) or `Body` (scripts). The structural keyword
/// words themselves (`then`/`elseif`/`else`) carry `ArgRole::Keyword`
/// so the semantic-token layer highlights them as keywords rather
/// than strings (mirrors Python's `_if_arg_roles`).
fn if_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    let mut roles = Vec::new();
    let mut i: usize = 0;
    let n = args.len();

    let push_role = |roles: &mut Vec<(u8, ArgRole)>, index: usize, role: ArgRole| {
        if let Ok(idx) = u8::try_from(index) {
            roles.push((idx, role));
        }
    };

    // First condition
    if i < n {
        push_role(&mut roles, i, ArgRole::Expr);
        i += 1;
    }
    // Optional 'then'
    if i < n && args[i] == "then" {
        push_role(&mut roles, i, ArgRole::Keyword);
        i += 1;
    }
    // First body
    if i < n {
        push_role(&mut roles, i, ArgRole::Body);
        i += 1;
    }

    while i < n {
        let kw = args[i];
        if kw == "elseif" {
            push_role(&mut roles, i, ArgRole::Keyword);
            i += 1;
            if i < n {
                push_role(&mut roles, i, ArgRole::Expr);
                i += 1;
            }
            if i < n && args[i] == "then" {
                push_role(&mut roles, i, ArgRole::Keyword);
                i += 1;
            }
            if i < n {
                push_role(&mut roles, i, ArgRole::Body);
                i += 1;
            }
            continue;
        }
        if kw == "else" {
            push_role(&mut roles, i, ArgRole::Keyword);
            if i + 1 < n {
                push_role(&mut roles, i + 1, ArgRole::Body);
            }
            break;
        }
        // Implicit else: trailing word with no keyword
        if i == n - 1 {
            push_role(&mut roles, i, ArgRole::Body);
            break;
        }
        i += 1;
    }
    roles
}

/// Command spec for `if`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "if",
        traits: Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::CONTROL_FLOW
            | Traits::LANGUAGE_KEYWORD
            | Traits::HAS_BOOLEAN_COND
            | Traits::NEVER_INLINE_BODY,
        arity: Arity::at_least(2),
        arg_role_resolver: Some(if_arg_roles),
        lowering_hook: Some(crate::hooks::LoweringHookId::If),
        return_type: Some(TclType::String),
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::Boolean),
                shimmers: true,
            },
        )],
        hover: Some(HoverSnippet {
            summary: "Conditional execution with optional elseif/else branches.",
            synopsis: &[
                "if expr1 ?then? body1 ?elseif expr2 ?then? body2 ...? ?else? ?bodyN?",
                "if expr1 ?then? body1 ?elseif expr2 ?then? body2 ...? ?else bodyN?",
            ],
            snippet: "Expressions are evaluated left-to-right until a true branch is selected.",
            source: "Tcl if(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
