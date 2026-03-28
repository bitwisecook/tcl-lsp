set arr(key) "value"
set ${dynamic} 1
set ::ns::var $arr(key)
puts [set x]
