# Regression guards for Copilot review findings on PR #180.
#
# 1. Nested ``catch`` must surface the outer body's last value into
#    its result variable.  Previously the outer's last statement was
#    an ``IRCatch`` — ``_emit_stmt_keep_result`` had no case for
#    that node, fell to the default, and pushed a null TclObj.  So
#    ``catch {catch {error x} m1} m2`` set ``m2 == ""`` instead of
#    the inner catch's return code.
set rc [catch { catch {error inner} m1 } m2]
if {$rc != 0} { error "outer rc should be 0, got $rc" }
if {$m1 ne "inner"} { error "m1 should be 'inner', got '$m1'" }
if {$m2 ne "1"} { error "m2 should be '1' (inner catch ret), got '$m2'" }

# Triple-nested: middle catch returns 1 (inner caught), outer
# catch returns 0 (middle caught implicitly) with m3 = "1" because
# the outer body's last stmt IS the middle catch whose leave
# returned 1.
set rc3 [catch { catch { catch {error x} a } b } c]
if {$rc3 != 0} { error "rc3 should be 0" }
if {$a ne "x"}  { error "a" }
if {$b ne "1"} { error "b = $b" }
if {$c ne "1"} { error "c = $c" }

# ``set x 42`` as catch-body-last-stmt stores 42 in the result var.
set rc4 [catch {set y 99} m4]
if {$m4 != 99} { error "set-body result lost: $m4" }

# ``dict create $k $v`` with whitespace-containing values must
# preserve the values (tcl_lappend, not tcl_concat).
set k "  key with spaces  "
set v "  value with spaces  "
set d [dict create $k $v]
if {[llength $d] != 2} { error "dict create dropped whitespace: $d" }
if {[dict get $d $k] ne $v} { error "dict create mangled value" }

# ``lreverse`` / ``linsert`` / ``lreplace`` preserve nested-list
# braces so sub-lists don't flatten into siblings.
if {[lreverse {{a b} c}] ne "c {a b}"} { error "lreverse flattened nested" }
if {[linsert {{a b} d} 1 X] ne "{a b} X d"} { error "linsert flattened" }
if {[lreplace {{a b} c d} 1 1 X] ne "{a b} X d"} { error "lreplace flattened" }
return ok
