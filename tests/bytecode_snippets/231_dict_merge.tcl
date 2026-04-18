# ``dict merge ?d1 d2 ...?`` returns a dict with entries from every
# source; source dicts later in the arg list win on duplicate keys.
# Previously there was no runtime helper and no compiler dispatch,
# so the command returned empty.
if {[dict merge] ne ""} { error "zero-arg merge" }
if {[dict merge {a 1}] ne "a 1"} { error "one-arg merge" }
if {[dict merge {a 1} {b 2}] ne "a 1 b 2"} { error "two disjoint" }
# Later source wins for duplicate keys.
if {[dict merge {a 1 b 2} {b 3 c 4}] ne "a 1 b 3 c 4"} {
    error "overlap: [dict merge {a 1 b 2} {b 3 c 4}]"
}
if {[dict merge {a 1} {b 2} {a 3}] ne "a 3 b 2"} {
    error "three dicts: [dict merge {a 1} {b 2} {a 3}]"
}
return ok
