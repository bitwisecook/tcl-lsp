# -all and -start restart the match with no left context: \A, \m and \y see the restart point as the string start
puts [regsub -all {\A.} abc X]
puts [regsub -all {\m.} "ab cd" X]
puts [regsub -all {\y} "ab cd" |]
puts [regexp -all -inline {\m.} "ab cd"]
puts [regexp -inline -start 1 {\A.} abc]
puts [regexp -inline -start 1 {^.} abc]
puts [regsub -all {^.} abc X]
puts [regexp -all -indices -inline {\m\w} "ab cd"]
