# The safe:: package: safe::interpCreate with access paths, tokenised paths.
set s [safe::interpCreate]
puts "issafe=[interp issafe $s]"
set cfg [safe::interpConfigure $s]
puts "has-accessPath=[expr {[lsearch -exact $cfg -accessPath] >= 0}]"
puts "statics=[safe::interpConfigure $s -statics]"
puts "token=[string match {$p(:*:)} [lindex [interp eval $s {set ::auto_path}] 0]]"
puts "source-is-alias=[expr {[lsearch -exact [interp aliases $s] source] >= 0}]"
puts "exit-is-alias=[expr {[lsearch -exact [interp aliases $s] exit] >= 0}]"
safe::interpDelete $s
puts "exists=[interp exists $s]"
