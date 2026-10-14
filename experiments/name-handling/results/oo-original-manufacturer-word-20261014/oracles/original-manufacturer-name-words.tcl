set name CHOSEN
proc record {value} {puts [list ORIGINAL_VALUE $value]}
record $name
record ${name}
record ::$name
record ::${name}
record "::$name"
record \x3a\x3a$name
record {$name}
record {::$name}
record :::$name
record prefix$name
record \$name
