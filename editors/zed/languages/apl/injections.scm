; A `[ … ]` bracket expression in APL *is* Tcl, and the tcl-lsp server walks it
; as Tcl (with the f5-iapps dialect). Re-parsing it as Tcl here keeps Zed's
; tree-sitter paint in step with the server's semantic tokens.

((tcl_expression) @injection.content
 (#set! injection.language "tcl")
 (#set! injection.include-children))
