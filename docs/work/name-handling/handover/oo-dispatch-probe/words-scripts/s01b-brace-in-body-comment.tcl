set code {proc p {} {
    # } brace in comment inside body
    return ok
}}
puts [list [catch {eval $code} m] $m [info complete $code]]
