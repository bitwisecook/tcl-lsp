# encoding system is process state; changing it affects encoding convertto with no name
set old [encoding system]
encoding system iso8859-1
puts "now: [encoding system]"
binary scan [encoding convertto "\u00e9"] H* h
puts "default convertto: $h"
encoding system utf-8
binary scan [encoding convertto "\u00e9"] H* h
puts "utf-8 convertto: $h"
encoding system $old
puts "bad: [catch {encoding system nosuch} m] $m"
puts "bad convert: [catch {encoding convertto nosuch x} m] $m"
