set object HANDLE
set {two words} GROUPED
set alias $object
puts [list bare [string equal $alias HANDLE]]
set alias "$object"
puts [list quoted [string equal $alias HANDLE]]
set alias {$object}
puts [list braced [string equal $alias {$object}]]
set alias {[C new]}
puts [list inert_command [string equal $alias {[C new]}]]
set alias prefix$object
puts [list compound [string equal $alias prefixHANDLE]]
set alias "${two words}"
puts [list grouped_reference [string equal $alias GROUPED]]
