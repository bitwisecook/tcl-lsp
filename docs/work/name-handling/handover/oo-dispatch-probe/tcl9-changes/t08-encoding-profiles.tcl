# TIP 656/657: strict encoding profile; identity and binary encodings removed
puts "identity: [expr {"identity" in [encoding names]}]"
puts "binary: [catch {encoding convertto binary abc} r] $r"
puts "strict: [catch {encoding convertfrom -profile strict utf-8 "\xC0\x80"} r] $r"
puts "default: [catch {string length [encoding convertfrom utf-8 "a\xFFb"]} r] $r"
puts "failindex: [catch {encoding convertfrom -failindex fi utf-8 "ab\xFF"} r] $r [info exists fi]"
