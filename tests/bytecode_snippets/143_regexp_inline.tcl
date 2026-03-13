set result [regexp -inline {(\w+)\s+(\w+)} "hello world"]
set all [regexp -all -inline {\d+} "a1b23c456"]
