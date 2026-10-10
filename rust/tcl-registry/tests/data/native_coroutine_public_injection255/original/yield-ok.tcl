if {[llength [info commands coroinject]] == 0} {
    set result UNAVAILABLE
} else {
    set log {}
    proc inject {tag mode kind value} {
        lappend ::log [list $tag $kind $value]
        if {$mode eq "error"} {error $tag}
        return $tag
    }
    coroutine c apply {{} {set ::value [yield ready]; return $::value}}
    coroinject c inject A ok
    coroinject c inject B ok
    set completion [catch {c ORIGINAL} result options]
    list $completion $result $log
}
