; An `ltm rule` body *is* Tcl — an iRule — and the tcl-lsp server walks it with
; the f5-irules dialect. Re-parsing it as Tcl here means a rule inside a config
; reads exactly as it does in a standalone `.irul`, instead of as flat config.
;
; `rule_body` is produced by the grammar's external scanner, which tracks Tcl's
; own brace rule (braces balance unless backslash-escaped), so the node's byte
; range is the body and nothing else — which is what the injection needs.

((rule_body) @injection.content
 (#set! injection.language "tcl"))
