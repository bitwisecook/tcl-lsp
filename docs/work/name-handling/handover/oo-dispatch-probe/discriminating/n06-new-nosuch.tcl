oo::class create C { method m {a} {} }
set o [C new]
$o nosuch
