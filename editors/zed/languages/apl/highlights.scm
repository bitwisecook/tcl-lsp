; APL (F5 iApp presentation) highlights for Zed.
;
; Backed by `grammars/tree-sitter-apl`. Before this, Zed's APL language pointed
; at the *Tcl* grammar and shipped no query file at all, so an APL file got no
; highlighting and — had it been queried — a garbage parse tree (issue #903).
;
; The captures mirror the semantic-token types the server emits for APL, so the
; tree-sitter paint and the LSP paint agree rather than fighting.

(comment) @comment

; #include / #inline
(include (directive) @keyword)

(define "define" @keyword)
(define name: (identifier) @function.definition)
(define type: (identifier) @type)

(section "section" @keyword.control)
(section name: (identifier) @label)

(table "table" @keyword.control)
(table name: (identifier) @label)

(text_block "text" @keyword.control)
(text_entry target: (dotted_name) @property)

(optional_block "optional" @keyword.control)
(binary_expression operator: _ @operator)

; A field's type may be user-defined, so the grammar accepts any identifier and
; the built-in types are picked out here — the same split the Tcl queries use
; for built-in commands.
(field
  type: (identifier) @type.builtin
  (#any-of? @type.builtin
    "addrport" "choice" "disena" "editchoice" "enadis" "enadisdry"
    "falsetrue" "indefint" "message" "multichoice" "noyes" "password"
    "string" "tcpprof" "truefalse" "yesno"))

(field type: (identifier) @type)
(field name: (identifier) @property)

(message_field "message" @type.builtin)

(attribute ["default" "display" "required" "validator"] @attribute)

; The validators APL ships with, named inside `validator "…"`. Only in that
; position — a bare `Number` in `display "Number"` is an ordinary string.
(attribute
  "validator"
  value: (string) @constant.builtin
  (#match? @constant.builtin "^\"(FQDN|IpAddress|IpOrFqdn|NonNegativeNumber|Number|PortNumber)\"$"))

(choice_mapping "=>" @operator)
(choice_mapping label: (string) @string.special)

(dotted_name) @property
(variable_substitution) @variable
(string) @string
(escape_sequence) @string.escape
(number) @number

["{" "}" "(" ")" "[" "]"] @punctuation.bracket
["," "."] @punctuation.delimiter
