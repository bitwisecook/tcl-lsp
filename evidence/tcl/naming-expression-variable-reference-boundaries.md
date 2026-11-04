# Tcl execution evidence

## Question

> A whole expression variable reference needs its selected public brace/parenthesis boundary. Literal dollar bytes, scalar unmatched parentheses and a combined closed array spelling cannot share a sigil-stripping or first-close rule across all releases.

> Which public expr lookup completion/result is observed for each original unmatched scalar parenthesis, closed combined/separate array spelling, repeated parentheses, literal dollar name and first-close versus nested-brace reference?

## Assumptions

Assumptions were not separately recorded in the retained capture.

Recorded scope and limits:

> One unchanged ASCII LF source-file CLI is executed independently on each pinned provider. Eight original labelled expression arguments and declared variable bytes, complete stdout/stderr/receipt and required executable/archive/header/build/source pins remain exact. The source contains its own catch and actual META report; no added eval/catch/version envelope is used. BIG-IP is not tested. The selected-reference-root source/API contract remains independent, and a linked Runtime whole-source comparison does not assert that it has executed. The linked shared Syntax diagnostic owner compares the exact retained C8.5/C8.6 nested-reference bareword-c message before a later rejected brace. Text/profile, actual expression grammar, original counted-byte and checked parser paths share typed lexical-prefix selection; an unfinished prefix cannot claim EOF before a later lexical error. These are source/API message projections and do not add a Native provider execution, private AST/object/cache observer or generic diagnostic-order theorem.

## Script

```tcl-visible
# Fixed ASCII LF source file. Each hex field is decoded by binary format H*
# into the reached expr argument or the exact declared variable name. This is
# an explicit constructed Tcl value channel, not opaque source-byte or native
# object/header/compiler-state admission. Public completion and result only.
puts [list META patchlevel [info patchlevel]]
proc expression_reference_probe {label name_hex expr_hex value} {
    set name [binary format H* $name_hex]
    set expression [binary format H* $expr_hex]
    set $name $value
    set code [catch [list expr $expression] result]
    binary scan $expression H* expression_bytes
    binary scan $result H* result_bytes
    puts [list EXPR label $label expression_hex $expression_bytes code $code result_hex $result_bytes]
    unset $name
}
expression_reference_probe unmatched_scalar_open 7363616c6172286f70656e 247b7363616c6172286f70656e7d 11
expression_reference_probe scalar_parenthesis_tail 7363616c6172286f70656e297461696c 247b7363616c6172286f70656e297461696c7d 12
expression_reference_probe combined_closed_element 617272286b657929 247b617272286b6579297d 21
expression_reference_probe separate_closed_element 617272286b657929 24617272286b657929 21
expression_reference_probe combined_multiple_parentheses 61727228696e6e657229286b657929 247b61727228696e6e657229286b6579297d 22
expression_reference_probe first_close_reference 617b62 247b617b627d 31
expression_reference_probe nested_close_reference 617b627d63 247b617b627d637d 32
expression_reference_probe literal_dollar_scalar 63617368246e616d65 247b63617368246e616d657d 41
```

## Results

### tcl8.4

- Recorded version: 8.4.20
- Recorded input channel: Exact ASCII LF source-file CLI. binary format H* explicitly constructs the declared variable name and reached expr argument. Public catch completion/result are sampled; no private parsed AST, table/header/cache, opaque source-byte, execution-entry or BIG-IP claim.
- Exit status: 0
- Standard output:

```text-visible
META patchlevel 8.4.20
EXPR label unmatched_scalar_open expression_hex 247b7363616c6172286f70656e7d code 0 result_hex 3131
EXPR label scalar_parenthesis_tail expression_hex 247b7363616c6172286f70656e297461696c7d code 0 result_hex 3132
EXPR label combined_closed_element expression_hex 247b617272286b6579297d code 0 result_hex 3231
EXPR label separate_closed_element expression_hex 24617272286b657929 code 0 result_hex 3231
EXPR label combined_multiple_parentheses expression_hex 247b61727228696e6e657229286b6579297d code 0 result_hex 3232
EXPR label first_close_reference expression_hex 247b617b627d code 0 result_hex 3331
EXPR label nested_close_reference expression_hex 247b617b627d637d code 1 result_hex 73796e746178206572726f7220696e2065787072657373696f6e2022247b617b627d637d223a20657874726120746f6b656e7320617420656e64206f662065787072657373696f6e
EXPR label literal_dollar_scalar expression_hex 247b63617368246e616d657d code 0 result_hex 3431
```

- Standard error: empty.

### tcl8.5

- Recorded version: 8.5.19
- Recorded input channel: Exact ASCII LF source-file CLI. binary format H* explicitly constructs the declared variable name and reached expr argument. Public catch completion/result are sampled; no private parsed AST, table/header/cache, opaque source-byte, execution-entry or BIG-IP claim.
- Exit status: 0
- Standard output:

```text-visible
META patchlevel 8.5.19
EXPR label unmatched_scalar_open expression_hex 247b7363616c6172286f70656e7d code 0 result_hex 3131
EXPR label scalar_parenthesis_tail expression_hex 247b7363616c6172286f70656e297461696c7d code 0 result_hex 3132
EXPR label combined_closed_element expression_hex 247b617272286b6579297d code 0 result_hex 3231
EXPR label separate_closed_element expression_hex 24617272286b657929 code 0 result_hex 3231
EXPR label combined_multiple_parentheses expression_hex 247b61727228696e6e657229286b6579297d code 0 result_hex 3232
EXPR label first_close_reference expression_hex 247b617b627d code 0 result_hex 3331
EXPR label nested_close_reference expression_hex 247b617b627d637d code 1 result_hex 696e76616c69642062617265776f7264202263220a696e2065787072657373696f6e2022247b617b627d637d223b0a73686f756c642062652022246322206f7220227b637d22206f72202263282e2e2e2922206f72202e2e2e
EXPR label literal_dollar_scalar expression_hex 247b63617368246e616d657d code 0 result_hex 3431
```

- Standard error: empty.

### tcl8.6

- Recorded version: 8.6.18
- Recorded input channel: Exact ASCII LF source-file CLI. binary format H* explicitly constructs the declared variable name and reached expr argument. Public catch completion/result are sampled; no private parsed AST, table/header/cache, opaque source-byte, execution-entry or BIG-IP claim.
- Exit status: 0
- Standard output:

```text-visible
META patchlevel 8.6.18
EXPR label unmatched_scalar_open expression_hex 247b7363616c6172286f70656e7d code 0 result_hex 3131
EXPR label scalar_parenthesis_tail expression_hex 247b7363616c6172286f70656e297461696c7d code 0 result_hex 3132
EXPR label combined_closed_element expression_hex 247b617272286b6579297d code 0 result_hex 3231
EXPR label separate_closed_element expression_hex 24617272286b657929 code 0 result_hex 3231
EXPR label combined_multiple_parentheses expression_hex 247b61727228696e6e657229286b6579297d code 0 result_hex 3232
EXPR label first_close_reference expression_hex 247b617b627d code 0 result_hex 3331
EXPR label nested_close_reference expression_hex 247b617b627d637d code 1 result_hex 696e76616c69642062617265776f7264202263220a696e2065787072657373696f6e2022247b617b627d637d223b0a73686f756c642062652022246322206f7220227b637d22206f72202263282e2e2e2922206f72202e2e2e
EXPR label literal_dollar_scalar expression_hex 247b63617368246e616d657d code 0 result_hex 3431
```

- Standard error: empty.

### tcl9.0

- Recorded version: 9.0.4
- Recorded input channel: Exact ASCII LF source-file CLI. binary format H* explicitly constructs the declared variable name and reached expr argument. Public catch completion/result are sampled; no private parsed AST, table/header/cache, opaque source-byte, execution-entry or BIG-IP claim.
- Exit status: 0
- Standard output:

```text-visible
META patchlevel 9.0.4
EXPR label unmatched_scalar_open expression_hex 247b7363616c6172286f70656e7d code 0 result_hex 3131
EXPR label scalar_parenthesis_tail expression_hex 247b7363616c6172286f70656e297461696c7d code 0 result_hex 3132
EXPR label combined_closed_element expression_hex 247b617272286b6579297d code 0 result_hex 3231
EXPR label separate_closed_element expression_hex 24617272286b657929 code 0 result_hex 3231
EXPR label combined_multiple_parentheses expression_hex 247b61727228696e6e657229286b6579297d code 0 result_hex 3232
EXPR label first_close_reference expression_hex 247b617b627d code 1 result_hex 6d697373696e6720636c6f73652d627261636520666f72207661726961626c65206e616d650a696e2065787072657373696f6e2022247b617b627d22
EXPR label nested_close_reference expression_hex 247b617b627d637d code 0 result_hex 3332
EXPR label literal_dollar_scalar expression_hex 247b63617368246e616d657d code 0 result_hex 3431
```

- Standard error: empty.

### tcl9.1

- Recorded version: 9.1.0
- Recorded input channel: Exact ASCII LF source-file CLI. binary format H* explicitly constructs the declared variable name and reached expr argument. Public catch completion/result are sampled; no private parsed AST, table/header/cache, opaque source-byte, execution-entry or BIG-IP claim.
- Exit status: 0
- Standard output:

```text-visible
META patchlevel 9.1.0
EXPR label unmatched_scalar_open expression_hex 247b7363616c6172286f70656e7d code 0 result_hex 3131
EXPR label scalar_parenthesis_tail expression_hex 247b7363616c6172286f70656e297461696c7d code 0 result_hex 3132
EXPR label combined_closed_element expression_hex 247b617272286b6579297d code 0 result_hex 3231
EXPR label separate_closed_element expression_hex 24617272286b657929 code 0 result_hex 3231
EXPR label combined_multiple_parentheses expression_hex 247b61727228696e6e657229286b6579297d code 0 result_hex 3232
EXPR label first_close_reference expression_hex 247b617b627d code 1 result_hex 6d697373696e6720636c6f73652d627261636520666f72207661726961626c65206e616d650a696e2065787072657373696f6e2022247b617b627d22
EXPR label nested_close_reference expression_hex 247b617b627d637d code 0 result_hex 3332
EXPR label literal_dollar_scalar expression_hex 247b63617368246e616d657d code 0 result_hex 3431
```

- Standard error: empty.

### jim

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: Exact ASCII LF source-file CLI. binary format H* explicitly constructs the declared variable name and reached expr argument. Public catch completion/result are sampled; no private parsed AST, table/header/cache, opaque source-byte, execution-entry or BIG-IP claim.
- Exit status: 0
- Standard output:

```text-visible
META patchlevel 0.84-9-g5bac7c9
EXPR label unmatched_scalar_open expression_hex 247b7363616c6172286f70656e7d code 0 result_hex 3131
EXPR label scalar_parenthesis_tail expression_hex 247b7363616c6172286f70656e297461696c7d code 0 result_hex 3132
EXPR label combined_closed_element expression_hex 247b617272286b6579297d code 0 result_hex 3231
EXPR label separate_closed_element expression_hex 24617272286b657929 code 0 result_hex 3231
EXPR label combined_multiple_parentheses expression_hex 247b61727228696e6e657229286b6579297d code 0 result_hex 3232
EXPR label first_close_reference expression_hex 247b617b627d code 0 result_hex 3331
EXPR label nested_close_reference expression_hex 247b617b627d637d code 1 result_hex 73796e746178206572726f7220696e2065787072657373696f6e3a2022247b617b627d637d22
EXPR label literal_dollar_scalar expression_hex 247b63617368246e616d657d code 0 result_hex 3431
```

- Standard error: empty.
