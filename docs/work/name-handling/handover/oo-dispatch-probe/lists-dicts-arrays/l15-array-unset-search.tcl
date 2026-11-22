array set a {x 1 y 2 z 3}
array unset a {[xy]}
puts [lsort [array names a]]
set id [array startsearch a]
puts [list [array anymore a $id] [array nextelement a $id] [array anymore a $id]]
array donesearch a $id
