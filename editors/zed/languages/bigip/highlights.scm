; BIG-IP configuration highlights for Zed.
;
; DO NOT EDIT — the keyed-reference list below is generated from the BIG-IP
; configuration lexer's key table (`tcl_bigip::conf_tokens::KEYED_REFERENCES`)
; by `cargo xtask gen-bigip-grammars`; the rest is a static template in that
; generator. A match is lexical display only: it proves no object exists.
;
; Backed by `grammars/tree-sitter-bigip`. Without it, Zed's BIG-IP language
; would point at the *Tcl* grammar and ship no query file at all, so a
; `bigip.conf` would get no highlighting and a garbage parse tree.
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

; A key the server reads as naming its value's kind: the bare (unpathed) value
; is an object name, as the server types it. The key itself stays a property —
; the server types it as one.
(property
  name: (word) @_key
  value: (word) @type
  (#any-of? @_key
    "pool" "last-hop-pool" "snatpool" "default-pool" "monitor" "monitors"
    "profile" "profiles" "defaults-from" "vlan" "vlans" "vlan-group"
    "rule" "rules" "policy" "policies" "persist" "fallback-persistence"
    "snat-translation" "traffic-group" "device-group" "gateway" "destination" "virtual-address"
    "route-domain" "default-route-domain" "node" "members"))

(property name: (word) @property)

(ip_address) @number
(ip_address port: (port) @number)
(encrypted) @string.special
(string) @string
(escape_sequence) @string.escape
(number) @number
(word) @variable

["{" "}"] @punctuation.bracket
