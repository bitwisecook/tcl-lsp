# linsert / lreplace single-value forms.  Previously unsupported — the
# compiler routed them through the eval fallback which trapped.
if {[linsert {a b c} 1 X] ne "a X b c"} { error "linsert middle" }
if {[linsert {a b c} 0 X] ne "X a b c"} { error "linsert head" }
if {[linsert {a b c} 99 X] ne "a b c X"} { error "linsert clamp-end" }
if {[linsert {} 0 X] ne "X"} { error "linsert empty" }

if {[lreplace {a b c d e} 1 3 X] ne "a X e"} { error "lreplace range+val" }
if {[lreplace {a b c d e} 1 3] ne "a e"} { error "lreplace delete-range" }
if {[lreplace {a b c} 0 0] ne "b c"} { error "lreplace delete-head" }
if {[lreplace {a b c} 5 5 X] ne "a b c X"} { error "lreplace append-beyond" }
return ok
