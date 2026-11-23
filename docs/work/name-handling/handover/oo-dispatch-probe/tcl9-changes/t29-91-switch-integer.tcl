# 9.1 TIP 730: switch -integer
puts [catch {switch -integer 010 {8 {set r eight} 10 {set r ten} default {set r none}}} r]:$r
