# The switch-body trap: an odd word count from a '#' line is a hard error
proc f {v} {
    switch $v {
        # a comment
        a {return A}
        default {return D}
    }
}
puts [catch {f a} r]:$r
