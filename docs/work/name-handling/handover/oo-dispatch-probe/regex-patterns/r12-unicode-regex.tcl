# regex over non-ASCII and non-BMP text, character offsets
set s "\xe9t\xe9"
puts [regexp -indices -inline {t} $s]
puts [regexp -all -inline {[[:alpha:]]} $s]
puts [string length [regsub -all {.} "a\U0001F600b" x]]
puts [regexp -inline {.} "\U0001F600"]
puts [string match {?} "\U0001F600"]
puts [regexp -nocase {\xc9} "\xe9"]
