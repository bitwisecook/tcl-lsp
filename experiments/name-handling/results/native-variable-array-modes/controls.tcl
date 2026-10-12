proc observe {id script} {
    set status [catch {uplevel 1 $script} value]
    puts [list NH_CONTROL $id $status $value]
}
observe variable-root {variable}
observe variable-namespace {namespace eval ::nh {variable}}
observe variable-procedure {proc ::nh_proc {} {variable}; nh_proc}
array set ::nh_keys {os system {a b} spaced alpha one beta two -glob dash}
observe names-default {lsort [array names ::nh_keys]}
observe names-glob {array names ::nh_keys -glob os}
observe names-prefix-glob {array names ::nh_keys -g os}
observe names-exact {array names ::nh_keys -exact {a b}}
observe names-regexp {lsort [array names ::nh_keys -regexp {^(alpha|beta)$}]}
observe names-badmode {array names ::nh_keys -bad os}
observe names-threepattern {array names ::nh_keys pattern os}
observe names-too-many {array names ::nh_keys -glob os extra}
observe names-dashpattern {array names ::nh_keys -glob}
