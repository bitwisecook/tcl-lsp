foreach {$literal} {SIGIL} {}
puts [list literal-sigil [set {$literal}]]
foreach {[literal]} {BRACKET} {}
puts [list literal-command-name [set {[literal]}]]
foreach {{}} {EMPTY} {}
puts [list empty-name [set {}]]
foreach {{a b}} {SPACE} {}
puts [list grouped-name [set {a b}]]
foreach a\ b {FIRST SECOND} {}
puts [list escaped-word-list $a $b]
unset a
foreach a\x62 {HEX} {}
puts [list escaped-word-name $ab]
foreach {a($key)} {ELEMENT} {}
puts [list literal-index [set {a($key)}]]
set names {$literal}
foreach $names {DYNAMIC} {}
puts [list substituted-variable-list [set {$literal}]]
