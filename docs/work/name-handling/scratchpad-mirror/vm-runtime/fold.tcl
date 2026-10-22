proc hover {b} {foreach {k v} $b {if {$k eq "example"} {puts [list $v]}}}
proc command {n body} {eval $body}
command x {
    hover {
        example {add_probe_insertion_point -net {count_out_c[0]} \
    -driver {D} -pin {H5}}
    }
}
