# tcl-dialect: tcl8.4

# `dict` is Tcl 8.5+; nothing in this file shadows the bare global name, so
# this call must fire W002.
dict create a 1

# A namespace-scoped proc shadows the disabled builtin for unqualified calls
# resolved inside that namespace (current-namespace-then-global) — must NOT
# fire.
namespace eval ::ns {
    proc dict {args} { return $args }
    dict foo bar
}

# `interp alias` establishing the name is a real command exactly like a proc
# would be — must NOT fire.
interp alias {} aliasedDisabled {} list
proc use_alias {} {
    aliasedDisabled foo bar
}

# A forward-declared proc — defined *after* this call site textually, but the
# call only runs once the whole file has loaded (deferred inside another
# proc's body) — must NOT fire. `lmap` is Tcl 8.6+, also disabled under 8.4.
proc use_lmap {} {
    lmap x {1 2 3} {expr {$x * 2}}
}
proc lmap {args} { return $args }
