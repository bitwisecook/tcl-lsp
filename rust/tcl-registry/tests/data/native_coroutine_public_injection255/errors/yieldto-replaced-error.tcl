if {[llength [info commands coroinject]] == 0} {
    set result UNAVAILABLE
} else {
    set log {}
    proc inject {tag mode kind value} {
        lappend ::log [list $tag $kind $value]
        if {$mode eq "error"} {
            lappend ::log [list branch $tag error]
            error ERROR_$tag
        }
        return $tag
    }
    coroutine c apply {{} {set ::value [yieldto list ready]; return $::value}}
    coroinject c inject A ok
    coroinject c inject B error
    set completion [catch {c ORIGINAL} result options]
    list $completion $result $log
}
