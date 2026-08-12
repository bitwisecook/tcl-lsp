# Port of rust/tcl-registry/src/commands/tcl/foreach_.rs
#
# Exercises: stepped arity, a repeated-argument layout with an excluded
# trailing body, a Tcl `arg_role_resolver` hook body, a positional arg-type
# hint used as a uniform key rather than a source index, var_write_typing,
# and named native lowering/analyser hooks.

speclib tcl 1.0 {

command foreach {
    dialects {all-tcl f5-irules}
    traits {NOT_PROC_FACTORY BYTE_COMPILED CONTROL_FLOW LANGUAGE_KEYWORD
            HAS_LOOP_BODY NEVER_INLINE_BODY LOOP_LIST_HEADER}

    # `varList list ?varList list ...? body` — an odd count from 3 (n
    # varList/list pairs, n >= 1, plus 1 body).  Confirmed against tclsh
    # 8.6.14: `foreach a $l1 b $l2 body extra` (6 args) fails "wrong # args".
    arity 3.. -step 2

    # The `?varlist list?...` head repeats at every other word from 0; the
    # body — the last word — is excluded from the tail.  Declaring the
    # repeating head means no consumer re-derives the stride from the
    # command's name (issue #1185).
    repeat LoopVarList -from 0 -stride 2 -exclude-trailing 1

    # The last argument is always the body.
    arg_role_resolver {words ctx} {
        if {[llength $words] >= 3} {
            role [expr {[llength $words] - 1}] Body
        }
    }

    # Index 0 here is a fixed key, not a source-position argument index: the
    # CFG builder lowers a foreach header to a synthetic call whose args are
    # only the list arguments (the var-lists live in defs).  Every list
    # argument expects the same List intrep, so the shimmer pass reads this
    # one entry and applies it uniformly to every iterator group — including
    # the later ones a positional per-index table could not reach.
    arg 0 -type List -shimmers

    return_type      String
    var_write_typing {ElementsOf 0}

    lowering_hook -native Foreach
    analyser_hook -native Foreach

    side_effect Unknown -reads -writes

    form Default {foreach varlist1 list1 ?varlist2 list2 ...? body}

    hover {
        summary {Iterate over one or more lists, assigning loop variables from each.}
        synopsis {foreach varname list body}
        synopsis {foreach varlist1 list1 ?varlist2 list2 ...? body}
        description {In the simple form, varname takes on each value of list in turn and body runs once per value. In the general form, each varlist/list pair is handled independently: on every iteration, the variables of each varlist are assigned consecutive values from its corresponding list, as if by lindex. Iteration continues until every value from every list has been used exactly once — enough passes to exhaust the longest list — and a list too short for its varlist supplies empty strings for the missing elements on later passes. break and continue inside body behave exactly as they do in for. foreach itself always returns an empty string, regardless of what body does.}
        source {Tcl foreach(n)}
        example {foreach x {a b c} {
    puts $x
}
foreach {name value} {height 6 width 8} {
    puts "$name = $value"
}
foreach x {1 2 3} y {a b} {
    puts "$x $y"
}}
        returns {An empty string.}
    }
}

}
