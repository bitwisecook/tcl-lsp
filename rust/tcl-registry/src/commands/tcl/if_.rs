//! `if` — conditional execution with optional elseif/else clauses.

use crate::prelude::*;

/// Dynamic arg role resolver for `if`/`elseif`/`else` chains.
///
/// Walks the argument list recognising `then`, `elseif`, `else`
/// keywords and classifying each positional argument as either
/// `Expr` (conditions) or `Body` (scripts).
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
            i += 1;
            if i < n {
                push_role(&mut roles, i, ArgRole::Expr);
                i += 1;
            }
            if i < n && args[i] == "then" {
                i += 1;
            }
            if i < n {
                push_role(&mut roles, i, ArgRole::Body);
                i += 1;
            }
            continue;
        }
        if kw == "else" {
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
        traits: Traits::CONTROL_FLOW
            | Traits::LANGUAGE_KEYWORD
            | Traits::HAS_BOOLEAN_COND
            | Traits::NEVER_INLINE_BODY,
        arity: Arity::at_least(2),
        arg_role_resolver: Some(if_arg_roles),
        return_type: Some(TclType::String),
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::Boolean),
                shimmers: true,
            },
        )],
        hover: Some(HoverSnippet::brief(
            "Conditional execution.",
            &["if expr1 ?then? body1 ?elseif expr2 ?then? body2 ...? ?else bodyN?"],
            "Tcl if(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
