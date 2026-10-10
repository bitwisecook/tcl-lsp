if {![llength [info commands ::oo::class]]} {return NOT_APPLICABLE:stock_TclOO_unavailable}
oo::class create base
set code [catch {oo::class create abc {superclass base; rename abc def; error foo}} result options]
list $code $result [dict get $options -errorinfo] [info commands ::def] [info commands ::oo::define::def]
