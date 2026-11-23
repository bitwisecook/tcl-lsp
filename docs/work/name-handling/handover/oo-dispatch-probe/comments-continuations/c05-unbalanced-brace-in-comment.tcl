# A brace in a comment still counts when the enclosing braces are matched
puts [catch {eval {proc p {} {
    # unbalanced } here
    puts in-p
}}} r]:$r
