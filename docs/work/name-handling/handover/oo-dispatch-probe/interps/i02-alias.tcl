# interp alias between interpreters: the target runs in the TARGET interp
# (here the parent), so it sees the parent's variables, with the alias's
# prefix words prepended.
set counter 0
proc bump {by} { global counter; incr counter $by; return $counter }
set c [interp create]
interp alias $c bumpBy {} bump
interp alias $c bump5 {} bump 5
puts "a=[interp eval $c bumpBy 2]"
puts "b=[interp eval $c bump5]"
puts "counter=$counter"
puts "aliases=[lsort [interp aliases $c]]"
puts "target=[interp alias $c bump5]"
puts "interp-target=[interp target $c bump5]"
# alias inside one interpreter ({} to {})
interp alias {} twice {} bump 2
puts "self=[twice]"
# alias from the parent into the child
interp eval $c { proc where {} { return [info level 0] } }
interp alias {} inChild $c where
puts "into-child=[inChild]"
puts "into-child-extra=[catch {inChild extra} m] $m"
# delete an alias by aliasing to the empty list
interp alias $c bump5 {}
puts "after-del=[lsort [interp aliases $c]]"
interp delete $c
