# auto_index entries are evaluated by unknown/auto_load on first call.
set auto_index(autofoo) {proc autofoo {x} {return "autofoo $x"}}
puts "before: [info commands autofoo]"
puts [autofoo 1]
puts "after: [info commands autofoo]"
set auto_index(::nsauto::bar) {namespace eval ::nsauto {proc bar {} {return bar}}}
puts [::nsauto::bar]
puts "auto_load missing: [auto_load nothing_here]"
set auto_index(baz) {proc baz {} {return baz}}
puts "auto_load explicit: [auto_load baz] [baz]"
