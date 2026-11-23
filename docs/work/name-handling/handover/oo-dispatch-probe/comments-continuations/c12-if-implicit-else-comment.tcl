# '#' after an if body is the implicit else body, not a comment
puts [catch {eval {if 0 {puts then} # comment}} r]:$r
puts [catch {eval {if 0 {puts then} #}} r]:$r
