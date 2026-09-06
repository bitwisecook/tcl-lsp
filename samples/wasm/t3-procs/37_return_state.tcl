# T3: a compiled `return` records the pending return state the `return`
# command records, so a caught `return -level 2` earlier in the program cannot
# leak into a later procedure's return boundary (issue #1774).
#
# `leak` and `probe` keep the `catch` out of the top level, which would
# otherwise decline the entry point's own lowering and leave nothing bound.
proc p {} { return v }
proc q {} { return }
proc leak {} { catch {return -level 2 -code error boom} }
proc probe {} { leak ; return [list [catch {p} m] $m] }
proc probe2 {} { leak ; return [list [catch {q} m] $m] }
puts [probe]
puts [probe]
puts [probe2]
