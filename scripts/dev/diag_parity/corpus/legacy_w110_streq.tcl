set x foo
if {$x == "foo"} { puts yes }
while {$x != "bar"} { set x bar }
if {$x eq "ok"} { puts good }
