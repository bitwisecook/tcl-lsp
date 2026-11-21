package require itcl
itcl::class C {
    public variable size 1
    method get {} { return $size }
}
C c -size 3
puts [c get]
puts [c cget -size]
puts [catch {c configure -colour 1} m]:$m ;# BAD
