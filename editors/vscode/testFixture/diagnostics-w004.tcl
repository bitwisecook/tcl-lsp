# tcl-dialect: tcl8.6
# W004 - command option is not available in the active dialect

# `lsearch -stride` was added in Tcl 9.0; on tcl8.6 this should produce W004.
lsearch -stride 2 {a b c d} b

# The abbreviated subcommand `conf` (unique prefix of `configure`) must
# still be resolved, so the dialect-gated `-inputmode` (Tcl 9.0+) is
# still checked here.
chan conf $chan -inputmode raw

# A user proc that redefines `lsearch` really is what gets called — the
# builtin's dialect-restricted `-stride` no longer applies, so this must
# NOT produce a second W004.
proc lsearch {l args} {
    return $l
}
lsearch -stride 2 {a b c d}
