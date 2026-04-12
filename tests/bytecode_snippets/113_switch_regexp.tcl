set x "hello123"
switch -regexp $x {
    {^[0-9]+$} {set y "number"}
    {^[a-z]+[0-9]+$} {set y "mixed"}
    default {set y "other"}
}
