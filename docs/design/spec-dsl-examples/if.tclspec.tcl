# Port of rust/tcl-registry/src/commands/tcl/if_.rs
#
# Not one of the eight assigned ports: `if` is here because it is the only
# shipped consumer of `clause_shape_check`, and the DSL's whole answer to that
# field is to make it unnecessary.  One `clause_grammar` declaration replaces
# BOTH `if_arg_roles` and `check_if_shape` with no hook body at all.

speclib tcl 1.0 {

command if {
    dialects {all-tcl f5-irules}
    traits {NOT_PROC_FACTORY BYTE_COMPILED CONTROL_FLOW LANGUAGE_KEYWORD
            HAS_BOOLEAN_COND NEVER_INLINE_BODY BRANCH_SELECTED_BODY
            STRUCTURALLY_CHECKED_ARITY}

    # Purely descriptive (hover / hint text): the real minimum is enforced by
    # the clause grammar, which also covers the chain shape a range cannot say.
    arity 2..

    # `expr ?then? body (elseif expr ?then? body)* (?else? body)?`, exactly as
    # if(n) writes it.  This one declaration replaces BOTH `if_arg_roles` and
    # `check_if_shape`:
    #
    #  * `head` is matched positionally and its slots are NEVER keyword-matched
    #    — `if else {a}` is a well-formed `if` whose condition is the bareword
    #    `else`, which is what C Tcl's IfConditionCallback does.
    #  * a missing Expr slot yields MissingExpr{after}, a missing Body slot
    #    MissingBody{after}, and any word past the tail ExtraWords{first_extra}
    #    — the three defects clause_shape_check reports.
    #  * `?else?` (an optional introducing keyword) is what makes the implicit
    #    trailing body legal with no keyword at all, and `tail` being last is
    #    what makes anything after it an error.
    clause_grammar {
        head            {Expr ?then? Body}
        repeated elseif {Expr ?then? Body}
        tail     ?else? {Body}
    }

    lowering_hook -native If
    return_type String
    arg 0 -type Boolean -shimmers

    side_effect Unknown -reads -writes

    form Default {if expr1 ?then? body1 ?elseif expr2 ?then? body2 ...? ?else? ?bodyN?}

    hover {
        summary {Conditional execution with optional elseif/else branches.}
        synopsis {if expr1 ?then? body1 ?elseif expr2 ?then? body2 ...? ?else? ?bodyN?}
        synopsis {if expr1 ?then? body1 ?elseif expr2 ?then? body2 ...? ?else bodyN?}
        description {Each expr is evaluated left to right, the same way expr evaluates its argument, until one is true; that clause's body runs and no later expr or body is touched. `then` and `else` are optional noise words kept only for readability — `if {$x} then {body}` and `if {$x} {body}` are equivalent. A boolean value is either numeric (0 is false, anything else is true) or one of the strings true/yes/false/no. Any number of elseif clauses may appear, including none, and the final body may be introduced with `else` or left bare with no keyword at all; an `else` with no body is an error, but a bare trailing body needs no `else` to be recognised. With no true expr and no final body, `if` returns an empty string.}
        source {Tcl if(n)}
        example {if {$vbl == 1} {
    puts "vbl is one"
} elseif {$vbl == 2} {
    puts "vbl is two"
} else {
    puts "vbl is not one or two"
}}
        returns {The result of whichever body script ran, or an empty string if no expr was true and no final body was given.}
    }
}

}
