# Even word count: the 'comment' silently becomes a pattern/body pair
proc f {v} {
    switch -- $v {
        # puts
        a {return A}
        default {return D}
    }
}
puts [f a]:[f b]
puts [catch {f #} r]:$r
