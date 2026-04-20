# oo::define to modify an existing class
oo::class create Animal {
    method speak {} { return "..." }
}
oo::define Animal {
    method speak {} { return "roar" }
}
set obj [Animal new]
$obj speak
