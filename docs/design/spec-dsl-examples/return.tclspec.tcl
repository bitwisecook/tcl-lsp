# Port of rust/tcl-registry/src/commands/tcl/return_.rs.
#
# The worked example tricky-surfaces.md asks for by name: "the one shipped
# option-arity hook (`return -errorstack`) as the worked example of a hook
# *inside* an option row".  `return` is also the registry's only
# `context_gate`, and one of its 44 argument-role resolvers, so one port
# exercises three of the eight hook families plus the ninth (option-arity)
# that had no example at all.
#
# Exercises: the option-arity hook family and its calling convention
# (`ctx`'s `option` / `option-index` / `option-value-start`), a resolver
# hook whose whole job is a shape test, a context gate, per-option dialect
# gates on four of six options, an integer domain alongside a closed value
# set, per-value completion codes, and three dialect-gated `form` rows for
# one command.

speclib tcl 1.0 {

# `-code`'s five symbolic completion codes, each paired with its canonical
# integer equivalent (`return -code error …` ≡ `return -code 1 …`).  A
# named table because the values carry `-code`, which the inline
# `-values {…}` list cannot: verified against tclsh 8.6.14, `-code` ALSO
# accepts an arbitrary integer alongside these (the `-integer Any` on the
# option row below), so this set is completion/hover metadata, not
# exhaustive validation — which is why the row is `-closed` *and*
# `-integer Any` at the same time.
values return-codes {
    value ok       -code 0 -detail {Normal completion (TCL_OK).}
    value error    -code 1 -detail {Error return (TCL_ERROR).}
    value return   -code 2 -detail {Propagate TCL_RETURN.}
    value break    -code 3 -detail {Propagate TCL_BREAK.}
    value continue -code 4 -detail {Propagate TCL_CONTINUE.}
}

command return {
    # ALL_TCL.union(IRULES): a pure control-flow primitive with no
    # filesystem/process/network access, so every dialect hosting a real Tcl
    # core carries it unmodified.  Its legal OPTION SET still narrows per
    # dialect through the per-option gates below, and its ARGUMENT SHAPE
    # narrows further inside an iRules event body (the third `form` and the
    # `context_gate`) — neither of which is a whole-command dialect gate.
    dialects {all-tcl f5-irules}

    traits {FRAMELESS_RUNTIME BYTE_COMPILED LANGUAGE_KEYWORD TERMINATES_BLOCK
            NEEDS_START_CMD}

    arity ..
    return_type String

    # return_arg_roles(): the bare `return VALUE` form's single word is the
    # command's own Result, and nothing else is.  Restricted to that one
    # shape on purpose (issue #1303) — with options in play the word is no
    # longer simply "this command's result" (`return -code error $msg`
    # completes exceptionally with $msg as the error payload).  A leading `-`
    # is rejected even at arity 1, where real Tcl would take it as the
    # result: an unpaired option word is far likelier to be a truncated call.
    arg_role_resolver {words ctx} {
        if {[llength $words] != 1} return
        if {[string index [lindex $words 0] 0] eq "-"} return
        role 0 Result
    }

    # F5 return(1): directly inside a `when EVENT { … }` body, `return` takes
    # no arguments.  A proc — even one defined inside the same `ltm rule` —
    # lives outside an event structurally, so `in-event-body` is 0 there and
    # this gate never fires.
    context_gate {words ctx} {
        if {![dict get $ctx in-event-body]} return
        if {[llength $words] == 0} return
        reject {`return` takes no arguments directly inside an iRules event body; wrap the call in a proc to use -code/-level/-errorcode/etc.}
    }

    lowering_hook       -native Return
    inline_codegen_hook -native Return

    side_effect InterpState  -writes
    side_effect EventControl -writes -dialects f5-irules

    # -level is Tcl 8.5+, so this form's own dialects must say so rather than
    # inheriting the command's set, or it would claim `-level` is legal
    # syntax in Tcl 8.4 and iRules (embedded Tcl 8.4.6), which it is not.
    form Default {return ?-code code? ?-level level? ?result?} -dialects tcl8.5+
    # `{tcl8.4 f5-irules}`, not bare `tcl8.4`: these are separate dialect
    # bits, and this form — return inside an iRules-defined proc — is exactly
    # what such a proc uses.
    form Default {return ?-code code? ?-errorinfo info? ?-errorcode code? ?string?} -dialects {tcl8.4 f5-irules}
    form Default {return} -dialects f5-irules

    option -code -takes code -values-from return-codes -closed -integer Any \
        -detail {Exceptional return code: ok/error/return/break/continue, or an integer (5-0x3fffffff reserved for application use by convention, not enforced).}

    option -level -takes level -integer {Range 0 2147483647} -dialects tcl8.5+ \
        -detail {Stack levels up the code applies to (default 1). 0 means this `return` itself returns -code. Must be 0..=2147483647; a negative or larger value is a hard error (unlike -code's integer, which never errors in this range).}

    option -errorcode -takes list \
        -detail {Additional error info, merged into the errorCode global. Only meaningful with -code error; defaults to "NONE" when omitted there.}

    option -errorinfo -takes info \
        -detail {Initial stack trace, merged into the errorInfo global. Only meaningful with -code error; Tcl supplies its own default when omitted there.}

    # THE OPTION-ARITY HOOK.  Port of errorstack_value()
    # (return_.rs:131-144), the registry's only OptionArity::Hook.
    #
    # `words` is the whole argument list after the command name — the same
    # `args` the Rust takes — and `ctx`'s `option-value-start` is its
    # `start`, so the port is a line-for-line transcription with no index
    # arithmetic of its own.
    #
    # The Rust distinguishes three outcomes: value absent (consume 0,
    # invalid), value present but not a list (consume 1, invalid), value
    # present and odd-sized (consume 1, invalid).  `string is list` stands in
    # for `split_list_raw`'s Ok/Err — the sandbox has `string` and does not
    # have `catch`, and the two agree on exactly which words are lists.
    #
    # Real tclsh 8.6.14 rejects an odd-sized list regardless of arity and
    # always consumes exactly one word (`-errorstack a b c d` takes only
    # `a`), so this hook never adjusts the span for a present word — it
    # exists for the CONTENT check.
    option -errorstack -takes list -dialects tcl8.6+ \
        -detail {Initial error stack (must be an even-sized list). Only meaningful with -code error.} \
        -arity-hook {words ctx} {
            set start [dict get $ctx option-value-start]
            if {$start >= [llength $words]} {
                consume 0 -invalid {missing value for -errorstack}
                return
            }
            set word [lindex $words $start]
            if {![string is list $word]} {
                consume 1 -invalid {value must be a valid Tcl list}
                return
            }
            if {[llength $word] % 2 != 0} {
                consume 1 -invalid {value must be an even-sized list}
                return
            }
            consume 1
        }

    option -options -takes dict -dialects tcl8.5+ \
        -detail {Dictionary of additional option/value pairs, merged in as if each had been given directly.}

    hover {
        summary  {Return from the current procedure/script with optional control-code metadata.}
        synopsis {return ?result?}
        synopsis {return ?-code code? ?result?}
        synopsis {return ?option value ...? ?result?}
        description {With no options, immediately returns from the current procedure — or, inside a script evaluated by source, stops evaluating that script — with an empty result unless result is given. -code sets an exceptional completion code instead of the default ok; -errorinfo and -errorcode (plus -errorstack from Tcl 8.6) are honoured only when -code is error, each seeding the matching global (errorInfo/errorCode) and defaulting to Tcl's own trace or "NONE" when omitted. -level (Tcl 8.5+, default 1) counts call-stack levels up the code applies to; -level 0 makes this return itself complete with code immediately, which is how interp alias builds break/continue-alike commands. -options (Tcl 8.5+) merges a dict of option/value pairs in as if each had been passed directly — typically the options dict a catch just captured, to re-raise a caught error unchanged. Tcl 8.4 recognises only -code, -errorinfo, and -errorcode. In an iRules event body, return takes no arguments at all and exits only the current event invocation; outside an event body iRules still exposes just that 8.4-shaped option set, since its embedded core is Tcl 8.4.6.}
        source {Tcl return(n); F5 return(1)}
        # ONE `example` block, not three rows: the shipped `examples` string
        # separates its three examples with a BLANK line, and repeated
        # `example` rows join with a single newline (README, "Brace-quoted
        # prose").  The equivalence gate compares the string byte for byte,
        # so the blank lines have to be inside the braced word.
        example {proc safeDiv {a b} {
    if {$b == 0} {
        return -code error "cannot divide by zero"
    }
    return [expr {$a / $b}]
}

# Re-throw a caught error unchanged, preserving errorInfo/errorCode (Tcl 8.5+)
if {[catch {doWork} result options]} {
    return -options $options $result
}

# Build a break-alike via -level 0 (Tcl 8.5+)
interp alias {} Break {} return -level 0 -code break}
        returns {The result string that becomes the enclosing procedure's result (or, inside a script evaluated by source, that script's result). With -code other than the default ok, result instead becomes the payload of the resulting exceptional completion — e.g. the message text of a -code error, or the value a catch reports when it traps this return.}
    }
}

}
