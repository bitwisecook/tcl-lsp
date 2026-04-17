# Basic class creation with method
oo::class create Dog {
    method bark {} { return "woof" }
}
set obj [Dog new]
$obj bark
