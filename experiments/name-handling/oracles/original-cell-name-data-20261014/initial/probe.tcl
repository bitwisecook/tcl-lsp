set {} EMPTY
puts [list empty [set {}]]
set {$name} SIGIL
puts [list braced-sigil [set {$name}]]
set {[name]} BRACKET
puts [list braced-command [set {[name]}]]
set escaped\x4eame ESCAPED
puts [list escaped [set escapedName]]
set {a($key)} ELEMENT
puts [list literal-index [set {a($key)}]]
set {($key)} ROOTLESS
puts [list empty-array-root [set {($key)}]]
foreach {{for$name}} {LOOP} {}
puts [list foreach-name [set {for$name}]]
catch {error BOOM} {$caught}
puts [list catch-name [set {$caught}]]
puts [list expr-name [expr {${$name} eq "SIGIL"}]]
unset -nocomplain missing
puts [list dynamic-name-missing [catch {set $missing 1}]]
