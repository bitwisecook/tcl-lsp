# Comprehensive fixture for variable-introducing Tcl command forms.
# Each ``# PROBE_*`` marker is the target of a VS Code test that inserts
# a ``$`` probe right after the marker and asserts the completion offers
# the locals introduced by the surrounding command form.
#
# Keeping a separate proc per form keeps each completion query scoped
# to just the locals from that form (plus the proc's parameters).

namespace eval ::ctxtests {
    variable namespace_var 100

    proc test_variable {} {
        # ``variable`` declaration inside a proc binds the namespace var
        # ``namespace_var`` as a local alias.
        variable namespace_var
        # PROBE_VARIABLE
    }

    proc test_namespace_upvar {} {
        namespace upvar ::ctxtests namespace_var local_ns_var
        # PROBE_NS_UPVAR
    }

    proc test_lassign {} {
        lassign {1 2 3} la lb lc
        # PROBE_LASSIGN
    }

    proc test_regexp {} {
        regexp {(\w+)=(\w+)} "foo=bar" rx_full rx_key rx_value
        # PROBE_REGEXP
    }

    proc test_regsub {} {
        regsub {foo} "foobar" baz rs_out
        # PROBE_REGSUB
    }

    proc test_scan {} {
        scan "1 2 3" "%d %d %d" sx sy sz
        # PROBE_SCAN
    }

    proc test_catch {} {
        catch {error boom} catch_result catch_opts
        # PROBE_CATCH
    }

    proc test_try {} {
        try {
            error boom
        } on error {try_msg try_opts} {
            # PROBE_TRY
        }
    }

    proc test_dict_update {} {
        set du_dict {a 1 b 2}
        dict update du_dict a alias_a b alias_b {
            # PROBE_DICT_UPDATE
        }
    }

    proc test_uplevel_one {} {
        # ``uplevel 1`` runs its body in the dynamic caller's frame, so this
        # proc's ``up1_local`` is NOT in scope there — completion abstains on
        # the enclosing proc's locals while still offering the body's own.
        set up1_local 1
        uplevel 1 {
            set body_var 5
            # PROBE_UPLEVEL_ONE
        }
    }

    proc test_partials {greeting} {
        # Probe for partial-prefix tests (``$gre``, ``${gre``, midword,
        # bare ``$``).  Each VS Code test inserts its own probe text at
        # this marker so the cursor lands at a known spot inside the
        # proc body where ``$greeting`` is in scope.
        # PROBE_PARTIAL
    }
}

# W215 fixture: variable names that contain ``}`` or array indices that
# contain ``)`` are creatable but unreachable via $-substitution.  The
# analyser should emit one W215 per offending site.
set "weird}name" 1                                    ;# W215 on '}'
set "arr(weird)stuff)" 2                               ;# W215 on ')'

# W216 fixture: broken brace-form array-element references.
set arr(name) hello
set foo bar
set indirect [puts ${arr}(name)]                       ;# W216 -- ${arr}(foo)
set indirect2 [puts ${arr($foo)}]                      ;# W216 -- ${arr($foo)}
# W216 funny-name case: array name has a space, so the fix falls back
# to the [set "..."] indirection form (which the command parser still
# substitutes $-vars in).
set "funny name" 1
set indirect3 [puts ${funny name($foo)}]               ;# W216 -- funny-name fallback

