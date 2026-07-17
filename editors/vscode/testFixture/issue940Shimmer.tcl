# Issue #940: a pure list literal is a valid list, so reading it as one is a
# free first conversion — no S100/S101 shimmer. The committed case at the end
# confirms a genuine container shimmer still fires.
set fontSizes {10.0 12.0 16.0 24.0}
foreach size $fontSizes { puts $size }
set empty {}
foreach x $empty { puts $x }
set a 1
foreach b $a { puts $b }
set d {a 1 b 2}
dict for {k v} $d { puts "$k=$v" }
set committed [dict create a 1 b 2]
foreach y $committed { puts $y }
