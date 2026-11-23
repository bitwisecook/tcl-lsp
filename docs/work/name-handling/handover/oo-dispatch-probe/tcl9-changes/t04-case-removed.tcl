# Removed pre-8 'case' command in 9.0
puts [catch {case abc in a* {set r A} default {set r D}} r]:$r
