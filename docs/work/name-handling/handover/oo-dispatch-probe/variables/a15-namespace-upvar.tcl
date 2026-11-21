namespace eval ::cfg { variable limit 3 }
oo::class create C {
    method lim {} { namespace upvar ::cfg limit l; return $l }
}
puts [[C new] lim]
