# ``lreverse`` reverses element order; ``lrepeat N v`` produces N copies.
# Both were unsupported previously — the compiler routed them through
# the eval fallback which trapped with ``unsupported command``.
if {[lreverse {a b c d}] ne "d c b a"} { error "lreverse broke: [lreverse {a b c d}]" }
if {[lreverse {}] ne ""} { error "empty lreverse broke" }
if {[lrepeat 3 foo] ne "foo foo foo"} { error "lrepeat 3 foo broke" }
if {[lrepeat 0 foo] ne ""} { error "lrepeat 0 should be empty" }
if {[llength [lrepeat 4 x]] != 4} { error "lrepeat 4 x length wrong" }
return ok
