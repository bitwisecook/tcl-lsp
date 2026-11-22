package require http
oo::class create K
set o [K new]
proc t {label script} { set code [catch {uplevel #0 $script} m]; puts "$label => [expr {$code==1 ? "ERROR: [string range $m 0 70]" : "ok"}]" }
t "oo::class create C {} extra" {oo::class create C2 {} extra}
t "info object class o K (1+1)" [list info object class $o K]
t "info object vars o pat" [list info object vars $o p*]
t "file link -symbolic a b" {file delete -force la lb; close [open lb w]; file link -symbolic la lb}
t "http::meta tok name" {set tok [http::geturl http://127.0.0.1:1 -timeout 1] ; http::meta $tok x}
t "http::registerError sock msg" {http::registerError sock1 msg}
t "http::formatQuery" {http::formatQuery}
t "interp limit {} time -seconds 10 -granularity" {interp limit {} time -seconds {} -granularity}
t "puts -nonewline stdout x" {puts -nonewline stdout x}
t "zlib stream deflate -level 6 -dictionary d -header h" {zlib stream deflate -level 6 -dictionary d}
t "zlib push deflate stdout -level 6 -dictionary d -header h -limit 1" {set f [open zz w]; zlib push deflate $f -level 6 -dictionary dd; close $f}
t "encoding convertfrom -profile strict -failindex v utf-8 data" {encoding convertfrom -profile strict -failindex v utf-8 data}
t "tcltest cleanup" {package require tcltest; tcltest::cleanupTests 0}
oo::class create H { method m {} { link } }
t "link 0 args in method" {[H new] m}
