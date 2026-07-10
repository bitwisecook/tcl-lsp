# T100 - tainted data flowing into a code-execution / numeric-coercion sink

# Direct sink: tainted $cmd flows into eval — should produce T100.
set cmd [gets stdin]
eval $cmd

# Branch condition: $n is a direct numeric operand of `+` inside the `if`
# condition, evaluated exactly like any other braced expr — should also
# produce T100, even though it never reaches a bare `expr` statement.
set n [gets stdin]
if {$n + 1 > 5} {
    puts big
}

# Pure string comparison: `eq` never coerces, so this must NOT produce T100.
set who [gets stdin]
if {$who eq "admin"} {
    puts ok
}

# subst -nocommands disables the only hazard T100 warns about — must NOT
# produce T100 (the recommended fix for the subst code-injection case).
set template [gets stdin]
subst -nocommands $template
