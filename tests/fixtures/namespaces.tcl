# Namespace and array tests
namespace eval ::math {
    variable PI 3.14159

    proc area_circle {radius} {
        variable PI
        return [expr {$PI * $radius * $radius}]
    }

    proc area_rect {width height} {
        return [expr {$width * $height}]
    }
}

puts "Circle area: [::math::area_circle 5]"
puts "Rect area: [::math::area_rect 3 4]"

# Array usage
set colors(red) "#FF0000"
set colors(green) "#00FF00"
set colors(blue) "#0000FF"

foreach {name value} [array get colors] {
    puts "$name = $value"
}

# Switch
set fruit "apple"
switch $fruit {
    apple  { puts "It's an apple" }
    banana { puts "It's a banana" }
    default { puts "Unknown fruit" }
}

# Regexp
set text "Hello 123 World 456"
regexp {(\d+)} $text match num
puts "Found number: $num"
