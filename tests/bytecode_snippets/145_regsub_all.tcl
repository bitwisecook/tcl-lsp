set s "abc123def456"
regsub -all {\d+} $s {NUM} result
regsub -all -nocase {[aeiou]} "Hello World" {*} result2
