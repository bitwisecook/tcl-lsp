# T6: objects stored in lists/dicts and dispatched dynamically; class vars;
# class-level methods via the class object; copy.
oo::class create Point {
    variable x y
    constructor {px py} { set x $px; set y $py }
    method coords {} { list $x $y }
    method dist2 {other} {
        lassign [$other coords] ox oy
        expr {($x - $ox) ** 2 + ($y - $oy) ** 2}
    }
    method move {dx dy} { incr x $dx; incr y $dy; return [self] }
}
oo::objdefine Point method origin {} { my new 0 0 }
set pts {}
foreach {a b} {1 2 3 4 5 6} { lappend pts [Point new $a $b] }
set o [Point origin]
foreach p $pts { puts [$p dist2 $o] }
set q [oo::copy [lindex $pts 0]]
$q move 10 10
puts [[lindex $pts 0] coords]
puts [$q coords]
puts [llength [info class instances Point]]
