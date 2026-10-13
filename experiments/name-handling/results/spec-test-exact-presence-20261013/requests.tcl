::__spec_test::present :literal :literal
puts [list PRESENCE :literal $::__spec_test::skip]
::__spec_test::present ghost* {ghost\*}
puts [list PRESENCE ghost* $::__spec_test::skip]
::__spec_test::present quer? {quer\?}
puts [list PRESENCE quer? $::__spec_test::skip]
::__spec_test::present present* {present\*}
puts [list PRESENCE present* $::__spec_test::skip]
::__spec_test::present ::present* {::present\*}
puts [list PRESENCE ::present* $::__spec_test::skip]
::__spec_test::present ::::present* {::::present\*}
puts [list PRESENCE ::::present* $::__spec_test::skip]
::__spec_test::present {bracket[one]} {bracket\[one\]}
puts [list PRESENCE {bracket[one]} $::__spec_test::skip]
::__spec_test::present {slash\name} {slash\\name}
puts [list PRESENCE {slash\name} $::__spec_test::skip]
