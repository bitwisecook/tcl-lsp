# Regression for Copilot review on PR #180:
# tcl_cmd_list_insert / tcl_cmd_list_replace / tcl_cmd_list_repeat all
# raw-memcpy'd the inserted value into the result.  Values containing
# whitespace or braces would re-parse as multiple elements (or
# completely invalid lists), silently corrupting the result.  All three
# now route the value through ``list_elem_quote`` /
# ``list_elem_quote_nth`` so the output is a canonical Tcl list.
set spaced {a b c}
if {[linsert {x y} 1 $spaced] ne {x {a b c} y}} { error "linsert spaced" }
if {[llength [linsert {x y} 1 $spaced]] != 3} { error "linsert llength" }

if {[lreplace {x y z} 1 1 $spaced] ne {x {a b c} z}} { error "lreplace spaced" }
if {[llength [lreplace {x y z} 1 1 $spaced]] != 3} { error "lreplace llength" }

if {[lrepeat 3 $spaced] ne {{a b c} {a b c} {a b c}}} { error "lrepeat spaced" }
if {[llength [lrepeat 3 $spaced]] != 3} { error "lrepeat llength" }

# Empty value still produces N elements (each ``{}``).
if {[llength [lrepeat 3 {}]] != 3} { error "lrepeat empty llength" }

# Brace-containing value also survives.
set bracey "a{b}c"
if {[llength [linsert {x y} 1 $bracey]] != 3} { error "linsert brace llength" }
if {[lindex [linsert {x y} 1 $bracey] 1] ne $bracey} { error "linsert brace content" }

# Plain (no-quoting-needed) values stay plain — no spurious braces.
if {[linsert {a b} 0 X] ne "X a b"} { error "linsert plain" }
if {[lrepeat 4 X] ne "X X X X"} { error "lrepeat plain" }
return ok
