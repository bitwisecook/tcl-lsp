# ``string index`` / ``string range`` must accept ``end``, ``end-N``,
# ``end+N`` index tokens (same arithmetic as list indexing).  Previously
# the runtime called ``obj_get_int`` on the index which returned 0 for
# non-numeric strings, so ``string index foo end`` gave ``f`` only by
# accident on 1-char inputs and always gave the first char otherwise.
if {[string index abcdef end] ne "f"} { error "end broke" }
if {[string index abcdef end-1] ne "e"} { error "end-1 broke" }
if {[string index abcdef end-5] ne "a"} { error "end-5 broke" }
if {[string index abcdef 0] ne "a"} { error "0 broke" }
if {[string index abcdef 3] ne "d"} { error "3 broke" }

if {[string range abcdef 2 end] ne "cdef"} { error "range 2 end" }
if {[string range abcdef 0 end-1] ne "abcde"} { error "range 0 end-1" }
if {[string range abcdef end-2 end] ne "def"} { error "range end-2 end" }
if {[string range abcdef 0 2] ne "abc"} { error "range 0 2" }
return ok
