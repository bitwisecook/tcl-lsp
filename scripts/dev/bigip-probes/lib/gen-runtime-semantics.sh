#!/usr/bin/env bash
# Emit the RULE_INIT probes that verify N-rule (brace-line continuation)
# semantics by execution rather than by compile acceptance. Each logs
# "VERIFY <id> ..." to /var/log/ltm.  Usage: gen-runtime-semantics.sh <outdir>
set -euo pipefail
OUT="${1:?outdir}"; mkdir -p "$OUT"
gen() { id="$1"; { echo "ltm rule probe_vf_$id {"; echo "when RULE_INIT {"; cat;
        echo "log local0. \"VERIFY $id out=<\$out>\""; echo "}"; echo "}"; } > "$OUT/$id.conf"; }

gen if_nextline <<'EOF'
set out "NOTRUN"
if {1}
{
  set out "body-ran"
}
EOF
gen deep_split_if <<'EOF'
set out "NOTRUN"
if
{1}
{
  set out "body-ran"
}
EOF
gen while_nextline <<'EOF'
set n 1
while {$n < 30}
{
  set n [expr {$n + 30}]
}
set out "n=$n"
EOF
gen foreach_nextline <<'EOF'
set acc ""
foreach i {a b c}
{
  append acc $i
}
set out "acc=$acc"
EOF
gen for_split <<'EOF'
set acc ""
for
{set x 0}
{$x<3}
{incr x} {
  append acc $x
}
set out "acc=$acc"
EOF
gen switch_nextline <<'EOF'
set out "NOTRUN"
switch "a"
{
  a {set out "matched-a"}
  default {set out "matched-default"}
}
EOF
gen else_nextline <<'EOF'
set out "NOTRUN"
if {0} {
  set out "then-branch"
}
else {
  set out "else-branch"
}
EOF
gen elseif_nextline <<'EOF'
set out "NOTRUN"
if {0} {
  set out "then-branch"
}
elseif {1} {
  set out "elseif-branch"
}
EOF
# --- N4: a non-brace line terminates the command ---
gen blankline_between <<'EOF'
set out "NOTRUN"
if {1}

{
  set out "body-ran"
}
EOF
gen wsonly_line <<'EOF'
set out "NOTRUN"
if {1}
   
{
  set out "body-ran"
}
EOF
gen comment_between <<'EOF'
set out "NOTRUN"
if {1}
# comment between condition and body
{
  set out "body-ran"
}
EOF
gen else_after_blank <<'EOF'
set out "NOTRUN"
if {0} {
  set out "then"
}

else {
  set out "else-branch"
}
EOF
gen catch_optional_arg <<'EOF'
set out "NOTRUN"
catch {set zz 1}
errv
set out "reached-end"
EOF
gen if_then_normal_cmd <<'EOF'
set out "NOTRUN"
if {1} {
  set out "then"
}
set out "$out+after"
EOF
# --- N2: the rule is lexical, not command-specific. Same commands, both forms ---
gen lex_set_bare     <<<$'set\nq 5\nset out "q=$q"'
gen lex_set_brace    <<<$'set\n{q} 5\nset out "q=$q"'
gen lex_incr_bare    <<<$'set c 0\nincr\nc\nset out "c=$c"'
gen lex_incr_brace   <<<$'set c 0\nincr\n{c}\nset out "c=$c"'
gen lex_append_bare  <<<$'set s ""\nappend\ns ab\nset out "s=$s"'
gen lex_append_brace <<<$'set s ""\nappend\n{s} ab\nset out "s=$s"'
gen lex_list_bare    <<<$'set out [list\na b]'
gen lex_list_brace   <<<$'set out [list\n{a} b]'
gen lex_string_bare  <<<$'set out [string\nlength abc]'
gen lex_string_brace <<<$'set out [string\n{length} abc]'
gen lex_expr_brace   <<<$'set out [expr\n{1+1}]'
gen lex_expr_bare    <<<$'set out [expr\n1+1]'
gen lex_lindex_brace <<<$'set out [lindex\n{a b} 1]'
gen lex_lindex_bare  <<<$'set out [lindex\n"a b" 1]'
gen lex_llength_brace <<<$'set out [llength\n{a b}]'
gen lex_llength_bare  <<<$'set out [llength\n"a b"]'
# --- N2 unconditional: absorbed even when the command is already complete ---
gen unconditional_list <<<$'set out [list a b\n{c}]'
gen unconditional_nested <<<$'set out "NOTRUN"\nif {1} {\n  set out\n  {inner}\n}'
# --- expr sub-parser: adjacency is NOT a divergence (see controls/expr_ctl.tcl) ---
gen expr_adjacent_eq         <<<$'set out [expr {"a"eq"a"}]'
gen expr_adjacent_startswith <<<$'set out [expr {"abc"starts_with"a"}]'
gen expr_adjacent_cmdsub     <<<$'set out [expr {[string length "xy"]eq"2"}]'
echo "$(ls "$OUT"/*.conf | wc -l) iRules -> $OUT"
