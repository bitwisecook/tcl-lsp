puts [list [catch {::switch -exact -- key {key {set x good} default {set x no}}} s] $s [catch {::dict get {key found} key} d] $d [catch {::dict getwithdefault {key found} other fallback} f] $f]
if {![catch {namespace ensemble configure ::dict -map} mapping]} {puts [list dict-get-map [dict get $mapping get]]}
