# Port of rust/tcl-registry/src/commands/tcl/upvar_.rs
#
# The frame_effect hard case.  The descriptor itself turns out to be two
# closed enums, so it is fully declarative — the difficulty is not saying
# "how does this command cross frames", it is the STATE-TRANSITION descriptor
# hanging off it, whose Rust form carries a resolver function pointer.
#
# Resolution taken here: the alias transitions upvar_state_transitions()
# produces are *entirely determined* by the frame_effect it reads
# (`AliasPairs` + `ArityParity`), so the DSL says `resolver from-frame-effect`
# instead of carrying code, and declares the surrounding plain-data knobs
# (composition, widening, effect coverage, commit) directly.

speclib tcl 1.0 {

command upvar {
    # A pure variable-scoping primitive — no filesystem, process, or network
    # access — so every dialect that hosts a real Tcl core carries it, iRules
    # included, and no dialect pack redefines it.
    dialects {all-tcl f5-irules}

    traits {FRAMELESS_RUNTIME NOT_PROC_FACTORY BYTE_COMPILED LANGUAGE_KEYWORD
            CREATES_BARRIER CREATES_SCOPE_ALIAS CREATES_DYNAMIC_BARRIER
            FRAME_HASH_BUILTIN ALIASES_CALLER_FRAME}

    # At least one otherVar/myVar pair: `upvar` and `upvar x` both raise
    # "wrong # args", `upvar x y` does not (tclsh 8.6.14).  No upper bound.
    arity 2..
    return_type String

    # C Tcl reads the level word off the ARGUMENT COUNT PARITY
    # (Tcl_UpvarObjCmd tests objc), never off the word's text — so
    # `upvar 1 b` aliases a caller variable literally named `1`, while
    # `upvar $lvl a b` really does take `$lvl` as its level.
    #
    # (`uplevel` is the other spelling: `-level_word LeadingProbe
    #  -layout ScriptInSelectedFrame`.)
    frame_effect -level_word ArityParity -layout AliasPairs

    lowering_hook -native Upvar
    codegen_hook  -native Upvar
    analyser_hook -native Upvar
    xc_translatable no

    side_effect Variable -writes

    world_effects none

    state_transitions {
        composition    Extend
        argument_shape Positional
        # Each otherVar/myVar pair becomes a VariableCellAlias against the
        # frame the level word selects — which is exactly what `frame_effect`
        # above already says, so no resolver body is written.
        resolver from-frame-effect
        widen  -operands EveryArgument -domains {VariableCells VariableTraces}
        covers LegacyFrame -domains {VariableStore}
        covers {LegacySideEffect Variable} -domains {VariableStore}
        # Alias pairs are processed in order and can be observed through
        # traces before a later pair fails.
        commit MayCommitBeforeAbruptCompletion
    }

    # The SYNOPSIS is byte-identical in the 8.4, 8.5, 8.6, 9.0, and 9.1
    # manpages, so one dialect-unrestricted form covers every version.
    form Default {upvar ?level? otherVar myVar ?otherVar myVar ...?}

    hover {
        summary {Create link to variable in a different stack frame}
        synopsis {upvar ?level? otherVar myVar ?otherVar myVar ...?}
        description {Links each myVar in the current procedure to the variable named otherVar in the call frame named by level (or the global scope, when level is #0); afterwards, reading, writing, or unsetting myVar reads, writes, or unsets otherVar directly. otherVar need not exist beforehand — it is created, just like an ordinary variable, the first time myVar is referenced. myVar must not already exist as a variable when upvar runs, and is always treated as a plain variable name, never an array element: since Tcl 8.5, a myVar that looks like an array element (e.g. a(b)) is a hard error ("can't create a scalar variable that looks like an array element"); Tcl 8.4 instead silently created an ordinary scalar variable literally named that. otherVar itself may be a scalar, a whole array, or a single array element. level takes any uplevel-style form — a plain integer counts call frames up from the current one (each namespace eval body also counts as one frame), #N is an absolute frame number, and it defaults to 1 (the immediate caller) whenever the first otherVar doesn't itself look like a level specifier; a level outside the current call stack is a "bad level" error. There is no way to remove an upvar link short of leaving the procedure that created it, though a later upvar call can retarget myVar to a different otherVar. A variable trace on otherVar fires on accesses through myVar but is passed myVar's name, not otherVar's; when otherVar names one element of an array, Tcl 8.4 through 8.6 do not fire a whole-array trace on that array for accesses through myVar (only a trace on that specific element fires), while Tcl 9.0 and 9.1 pass the element name as the trace procedure's second argument, so a whole-array trace does observe the access.}
        source {Tcl upvar(n)}
        example {proc add2 name {
    upvar $name x
    set x [expr {$x + 2}]
}
set n 5
add2 n
puts $n

# level defaults to 1 (the caller); an explicit level reaches further up the stack
proc decr {varName {decrement 1}} {
    upvar 1 $varName var
    incr var [expr {-$decrement}]
}

# level #0 links straight to the global scope, regardless of call depth
proc bumpCounter {} {
    upvar #0 counter c
    incr c
}}
        returns {The empty string.}
    }
}

}
