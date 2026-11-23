# The switch-body trap: '#' lines are list words, not comments
proc f {v} {
    switch $v {
        # this is not a comment
        a {return A}
        default {return D}
    }
}
puts [catch {f a} r]:$r
