# ``lsort ?-switches? list`` — the runtime export is the no-switch form
# (string-compare ascending), so the codegen must still pass the
# trailing positional list when options are present rather than
# treating ``-integer``/``-decreasing``/... as the list itself.
if {[lsort -integer {3 1 2 10}] eq "-integer"} {
    error "lsort swallowed the option as the list — regression"
}
# The actual sort (string-compare for now) should still produce the
# elements.
set r [lsort -integer {3 1 2 10}]
if {[llength $r] != 4} { error "lsort -integer produced $r" }
if {[lsort {3 1 2}] ne "1 2 3"} { error "plain lsort broken" }
return ok
