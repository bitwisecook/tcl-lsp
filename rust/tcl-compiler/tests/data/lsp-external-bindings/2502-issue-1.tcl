set style [dict create \
    bg #ffffff \
    bc #cbd5e1 \
    bw 1 \
    radius {8 8 8 8} \
    pad {0 0 0 0} \
    shadow {} \
    snap 1]

dict with style {}

set radius [lmap r $radius {
    expr {min(1, 2 / 2.0, 3 / 2.0)}
}]

if {$bg ne ""} {
    # do something...
    set bg ""
}
