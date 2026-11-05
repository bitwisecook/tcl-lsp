# naming.variable.setter-original-child-expression-identity

Kind: `native-observation`

## Problem statement

A nested source command can be the stock expression handler, its interpreter alias or a same-spelled procedure returning literal data. The written expression label alone cannot distinguish their variable-read behavior.

## Question

What do the four exact direct, aliased and shadowed setter/expression source forms return under the six retained CLI providers?

## Conclusion

All five C providers return4 for direct and aliased expressions and VALUE for the same-spelled procedure through direct and aliased setters. Current Jim returns4 and VALUE for the direct forms, while both interpreter-alias attempts return the actual interp wrong-arity error. These results distinguish selected source child behavior without establishing compiler or private storage facts.

## Scope

Six original provider processes, each with an actual version row and four caught observations (24 observations total). Fixed ASCII LF source-file CLI input; observations run sequentially in the same fresh process, so alias_expression reuses input and alias_setter intentionally reuses the earlier expression shadow. The original authored source and executed version-prefixed source have separate retained SHA joins. Raw receipts call the catches independent; that sentence does not imply independent interpreter state. No opaque/counting-object input, compiler admission, physical read/frame/cell/cache/header, callback closure, Native Normal transfer or BIG-IP answer is established.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Exact ASCII LF source-file CLI; one fresh provider process runs four caught observations sequentially after its actual version row. Original source and the separately added version-prefix source are both retained. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Direct and aliased expression return4. The same-spelled procedure returnsVALUE through direct and aliased setters; its braced missing name is not evaluated by the stock expression handler.

### jim

Status: `observed`. Channel: Exact ASCII LF source-file CLI; one fresh provider process runs four caught observations sequentially after its actual version row. Original source and the separately added version-prefix source are both retained. Version: jim 0.84-9-g5bac7c9.

Direct builtin expression returns4; same-spelled procedure returnsVALUE. Both interpreter-alias attempts fail with the recorded interp arity usage; no successful alias read or setter is observed.

### bigip

Status: `not-tested`.

No appliance measurement of these source forms.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_setter_expression_original/8.6.18](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/8.6.18)
- [rust/tcl-registry/tests/data/native_setter_expression_original/9.1.0](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/9.1.0)
- [rust/tcl-registry/tests/data/native_setter_expression_original/8.4.20](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/8.4.20)
- [rust/tcl-registry/tests/data/native_setter_expression_original/9.0.4](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/9.0.4)
- [rust/tcl-registry/tests/data/native_setter_expression_original/8.5.19](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/8.5.19)
- [rust/tcl-registry/tests/data/native_setter_expression_original/jim](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/jim)
