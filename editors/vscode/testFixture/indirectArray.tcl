# FP-STY-12 fixture: braced indirect-array-element idiom ${var}(idx).
# `token` holds an array name, so the varname-position uses below are the
# legitimate indirect access idiom and must NOT fire W216 or W212.
set token ::http::1
set ${token}(status) eof
info exists ${token}(-pipeline)
unset ${token}(socketcoro)
# Value position: this IS a broken read for $arr(x) and MUST fire W216.
puts ${arr}(x)
