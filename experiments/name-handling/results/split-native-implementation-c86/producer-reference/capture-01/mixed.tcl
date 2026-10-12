puts [list RELEASE [info patchlevel]]
set v V; proc mark {} {return X}; set d "A\0$v[mark]\u0000B"
list $d
