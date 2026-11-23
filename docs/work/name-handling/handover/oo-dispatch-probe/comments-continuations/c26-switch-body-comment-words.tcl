# The switch-body trap: a 6-word "comment" pairs up as patterns and bodies
proc f {v} {
    switch $v {
        # this is not a comment
        a {return A}
        default {return D}
    }
}
puts [catch {f a} r]:$r
