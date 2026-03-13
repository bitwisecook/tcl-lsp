; Tcl highlights for Zed
; Adapted from upstream tree-sitter-tcl queries/tcl/highlights.scm

; Comments
(comment) @comment

; Strings and escapes
(quoted_word) @string
(escaped_character) @string.escape

; Numbers
(number) @number

((simple_word) @number
  (#match? @number "^[+-]?[0-9]+$"))

((simple_word) @boolean
  (#any-of? @boolean "true" "false"))

; Variables
(variable_substitution) @variable

(set (id) @variable)

(argument
  name: (_) @variable.parameter)

; Tcl built-in variables
((simple_word) @variable.builtin
  (#any-of? @variable.builtin
    "argc" "argv" "argv0" "auto_path" "env" "errorCode" "errorInfo"
    "tcl_interactive" "tcl_library" "tcl_nonwordchars" "tcl_patchLevel"
    "tcl_pkgPath" "tcl_platform" "tcl_precision" "tcl_rcFileName"
    "tcl_traceCompile" "tcl_traceExec" "tcl_wordchars" "tcl_version"))

; Procedure definitions
"proc" @keyword.function

(procedure
  name: (_) @function.definition)

; Control flow keywords
["if" "else" "elseif"] @keyword.control

["while" "foreach"] @keyword.control

["catch" "try" "finally"] @keyword.control

["set" "global" "namespace" "on"] @keyword

; Built-in commands
"expr" @function.builtin

(command
  name: (simple_word) @function.builtin
  (#any-of? @function.builtin
    "cd" "exec" "exit" "incr" "info" "join" "puts" "regexp" "regsub"
    "split" "subst" "trace" "source" "read" "gets" "open" "close"
    "flush" "seek" "tell" "eof" "fconfigure" "fcopy" "fileevent"
    "socket" "encoding" "binary" "chan"))

(command
  name: (simple_word) @keyword
  (#any-of? @keyword
    "append" "break" "catch" "continue" "default" "dict" "error"
    "eval" "global" "lappend" "lassign" "lindex" "linsert" "list"
    "llength" "lmap" "lrange" "lrepeat" "lreplace" "lreverse"
    "lsearch" "lset" "lsort" "package" "return" "trap" "throw"
    "variable" "unset" "upvar" "uplevel" "rename" "array" "string"
    "format" "scan" "clock" "file" "glob" "pid" "pwd" "load"
    "unload" "update" "vwait" "after" "interp" "apply" "tailcall"
    "coroutine" "yield" "zlib" "try"))

; Highlight unset / variable arguments as variables
(command
  name: (simple_word) @_kw
  arguments: (word_list) @variable
  (#any-of? @_kw "unset" "variable"))

; Generic command calls
(command
  name: (simple_word) @function)

; Operators
(unpack) @operator

["**" "/" "*" "%" "+" "-"
 "<<" ">>"
 ">" "<" ">=" "<="
 "==" "!="
 "eq" "ne"
 "in" "ni"
 "&" "^" "|"
 "&&" "||"
] @operator

; Punctuation
["{" "}" "[" "]" ";"] @punctuation.bracket
