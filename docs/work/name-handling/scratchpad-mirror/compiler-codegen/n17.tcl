namespace eval a { variable b NSB }
set a x
set a:b BAD
set {a:} LONE
set arr(x:b) good
set arr(BAD) bad
set arr(x:) lone-good
set arr(LONE) lone-bad
set arr(NSB) ns-good
set arr(x:::b) run3-bad
set arr(x::) sep-good
proc show {args} { puts [join $args |] }
show $arr($a:b) $arr(${a}:b) $arr($a:) $arr($a::b) [catch {set arr($a:::b)} m]$m $a::: [catch {set r $a::} m]$m
