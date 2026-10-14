puts [list provider [info patchlevel]]
set arr(KEY) ARRAY
set index KEY
set tail TAIL
set {$literal} SIGIL
set {} EMPTY
set value "$arr([set index])/$tail/$tail/${$literal}/${}"
puts [list nested $value]
set escaped "\$inert/\\$tail"
puts [list escaped $escaped]
set {a{b}c} NESTED
set "a{b" FIRST
set template {${a{b}c}/$tail}
set code [catch {subst $template} result]
puts [list closer $code $result]
set code [catch {set quoted "${a{b}c}/$tail"} result]
puts [list written_closer $code $result]
