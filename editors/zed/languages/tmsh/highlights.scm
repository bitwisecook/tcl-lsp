; BIG-IP configuration (tmsh) highlights for Zed.
;
; Backed by `grammars/tree-sitter-bigip`. Before this, Zed's TMSH language
; pointed at the *Tcl* grammar and shipped no query file at all, so a
; `bigip.conf` got no highlighting and a garbage parse tree (issue #903).
;
; The captures mirror the BIG-IP semantic-token types the server emits. Where
; the grammar cannot be as specific as the server (a bare `/Common/x` does not
; say whether it names a pool or a monitor) it stays generic rather than guess:
; less specific, never contradictory.

(comment) @comment

(module) @keyword
(object_type) @keyword

; The partition of an object path is a namespace, as the server types it.
(object_path partition: (path_component) @namespace)
(object_path port: (port) @number)
(object_path route_domain: (number) @constant)

; A pool member's or virtual's final path component *is* an address. Without
; this it would fall through to the generic object-name capture and be painted
; a different colour than the server's `ipAddress` — the one place these two
; layers could contradict each other outright.
((object_path name: (path_component) @number)
 (#match? @number "^([0-9]{1,3}\\.){3}[0-9]{1,3}$"))

(object_path name: (path_component) @type)

; A reference whose type is named by the preceding word — the one case where the
; grammar can be exactly as specific as the server, so it is.
(property
  name: (word) @keyword
  (#any-of? @keyword
    "pool" "monitor" "profile" "persistence" "vlan" "vlans" "interface"))

(property name: (word) @property)

(ip_address) @number
(ip_address port: (port) @number)
(encrypted) @string.special
(string) @string
(escape_sequence) @string.escape
(number) @number
(word) @variable

["{" "}"] @punctuation.bracket
