proc greet {name greeting punctuation} {
    puts "$greeting, $name$punctuation"
}

set who "Ada"
set salutation "Hello"
greet $who $salutation
#                     ^--- cursor
