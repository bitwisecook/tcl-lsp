set raw {$(1+2)}
puts "RAW=$raw"
set value "$(1+2)"
puts "EVALUATED=$value"
set library_code [catch {info library} library]
puts "LIBRARY_CODE=$library_code"
puts "LIBRARY=$library"
