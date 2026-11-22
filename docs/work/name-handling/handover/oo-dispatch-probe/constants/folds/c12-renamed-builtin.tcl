proc p {} {
    set a [string length abc]
    rename ::string ::_s
    proc ::string {args} { return 99 }
    set b [string length abc]
    rename ::string {}
    rename ::_s ::string
    return [list $a $b]
}
puts [p]
