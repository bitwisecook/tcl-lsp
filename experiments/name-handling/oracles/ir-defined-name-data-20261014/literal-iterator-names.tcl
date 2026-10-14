puts [list provider [info patchlevel]]
foreach {$iterator ${iterator} {} array(key)} {SIGIL BRACE EMPTY ELEMENT} {}
puts [list values [set {$iterator}] [set {${iterator}}] [set {}] [set array(key)]]
puts [list stripped_exists [info exists iterator]]
set index OTHER
set {arr($index)} LITERAL_INDEX
puts [list literal_index [set {arr($index)}] [info exists arr(OTHER)]]
