#!/usr/bin/env python3
"""Grouped evidence runs for the §12 open questions.

  evidence.py gen <outdir>              write <id>.conf/.delete/.traffic for drive.sh
  evidence.py render <drive-output> <evidence-dir> <scratch-dir>
                                        write one evidence/tcl/*.md per case

Each case is one iRule (one executed script). The per-case probes in gen.py
established that every cell loads on its own; these grouped rules are the
recorded evidence. Cases whose point is a load-time refusal are their own
rules.
"""
import pathlib
import re
import sys
import textwrap

VIP = "192.168.9.24"
BUILD = "BIG-IP 21.1.0.1 build 0.0.26 (Point release 1), VE, single NIC, 4 TMMs"
DATE = "2026-11-20 UTC"


def L(case, *fields):
    return 'log local0. "TCLLSP_OQ|' + case + "|" + "|".join(fields) + '"'


def rows_rule_init(case, rows):
    """RULE_INIT body: each (label, expression-or-script, kind) under catch."""
    out = ["when RULE_INIT {"]
    for label, script in rows:
        out.append(f"set c [catch {{{script}}} v]")
        out.append(L(case, label, "$c", "$v"))
    out.append("}")
    return "\n".join(out) + "\n"


def expr_rows(pairs):
    return [(label, f"expr {{{e}}}") for label, e in pairs]


CASES = []


def case(**kw):
    CASES.append(kw)


# ------------------------------------------------------------- operators
case(id="opmatches", slug="bigip-f5-matches-operator-semantics",
     title="What the iRule `matches` operator compares",
     question="""Does the iRule word operator `matches` compare for string equality, glob-match its right operand, or regex-match it, and do `matches_glob` and `matches_regex` behave as their names say? The analyser's authored evaluator treated `matches` as equality, and the measured `"abcd" matches "bc"` = 0 could not tell equality from an anchored glob. This scopes TMM expression semantics only; it does not cover `class match` or LTM policy operators.""",
     body=rows_rule_init("opmatches", expr_rows([
         ("eq", '"abc" matches "abc"'),
         ("substring", '"abcd" matches "bc"'),
         ("prefix", '"abcd" matches "ab"'),
         ("star", '"abc" matches "a*"'),
         ("star_literal", '"a*" matches "a*"'),
         ("class", '"b" matches "\\[abc\\]"'),
         ("dot", '"abc" matches "a.c"'),
         ("question", '"abc" matches "a?c"'),
         ("case", '"ABC" matches "a*"'),
         ("glob_star", '"abc" matches_glob "a*"'),
         ("glob_exact", '"abc" matches_glob "abc"'),
         ("regex_anchored", '"abc" matches_regex {^a.c$}'),
         ("regex_unanchored", '"xabcx" matches_regex {a.c}'),
     ])))

BIND = [
    ("c1", "1 & 3 contains 3"), ("c1a", "(1 & 3) contains 3"), ("c1b", "1 & (3 contains 3)"),
    ("c2", '"ab" contains "a" == 1'), ("c2a", '("ab" contains "a") == 1'), ("c2b", '"ab" contains ("a" == 1)'),
    ("c3", "1 == 2 contains 0"), ("c3a", "(1 == 2) contains 0"), ("c3b", "1 == (2 contains 0)"),
    ("c4", '"x0" contains 1 < 2'), ("c4a", '("x0" contains 1) < 2'), ("c4b", '"x0" contains (1 < 2)'),
    ("c5", "0 && 0 contains 0"), ("c5a", "(0 && 0) contains 0"), ("c5b", "0 && (0 contains 0)"),
    ("c6", '"12" starts_with 1 + 1'), ("c6a", '("12" starts_with 1) + 1'), ("c6b", '"12" starts_with (1 + 1)'),
    ("c7", '"21" ends_with 1 + 1'), ("c7a", '("21" ends_with 1) + 1'), ("c7b", '"21" ends_with (1 + 1)'),
    ("c8", "1 == 2 equals 0"), ("c8a", "(1 == 2) equals 0"), ("c8b", "1 == (2 equals 0)"),
    ("c9", "1 == 2 matches_glob 0"), ("c9a", "(1 == 2) matches_glob 0"), ("c9b", "1 == (2 matches_glob 0)"),
    ("c10", "1 == 2 matches_regex 0"), ("c10a", "(1 == 2) matches_regex 0"), ("c10b", "1 == (2 matches_regex 0)"),
    ("c11", '"abc" contains "b" contains 1'), ("c11a", '("abc" contains "b") contains 1'), ("c11b", '"abc" contains ("b" contains 1)'),
    ("c12", "1 == 2 matches 0"), ("c12a", "(1 == 2) matches 0"), ("c12b", "1 == (2 matches 0)"),
    ("c13", '"x0" starts_with 1 < 2'), ("c13a", '("x0" starts_with 1) < 2'), ("c13b", '"x0" starts_with (1 < 2)'),
]
case(id="opbind", slug="bigip-f5-word-operator-binding-power",
     title="Binding power of the iRule string word operators",
     question="""Where do `contains`, `starts_with`, `ends_with`, `equals`, `matches`, `matches_glob` and `matches_regex` bind relative to `==`, `<`, `+`, `&` and `&&`, and are they left-associative? Each bare expression is followed by its two explicit groupings; the bare result equals exactly one of them where the case discriminates. W154/W155 group these operators at the `==` level, which was unmeasured. Scope: TMM `expr` with braced literal expressions in `RULE_INIT`.""",
     body=rows_rule_init("opbind", expr_rows(BIND)))

LOGIC = [
    ("o1", "1 or 0 and 0"), ("o1a", "(1 or 0) and 0"), ("o1b", "1 or (0 and 0)"),
    ("o2", "0 and 0 == 0"), ("o2a", "(0 and 0) == 0"), ("o2b", "0 and (0 == 0)"),
    ("o3", "1 or 0 && 0"), ("o3a", "(1 or 0) && 0"), ("o3b", "1 or (0 && 0)"),
    ("o4", "1 || 0 and 0"), ("o4a", "(1 || 0) and 0"), ("o4b", "1 || (0 and 0)"),
    ("o5", "0 or 1 ? 5 : 6"), ("o5a", "(0 or 1) ? 5 : 6"), ("o5b", "0 or (1 ? 5 : 6)"),
    ("o7", "0 or 2"), ("o8", "0 and [error side]"), ("o9", "1 or [error side]"),
    ("n1", "not 2 == 1"), ("n1a", "(not 2) == 1"), ("n1b", "not (2 == 1)"),
    ("n2", "not 0 + 1"), ("n2a", "(not 0) + 1"), ("n2b", "not (0 + 1)"),
    ("n3", "not 10 contains 0"), ("n3a", "(not 10) contains 0"), ("n3b", "not (10 contains 0)"),
    ("n4", "!2 == 1"), ("n5", "not 1 and 0"), ("n5a", "(not 1) and 0"), ("n5b", "not (1 and 0)"),
    ("n6", "not 5"), ("n7", 'not "abc"'),
]
case(id="oplogic", slug="bigip-f5-and-or-not-binding-power",
     title="Binding power and short-circuit of iRule `and`, `or` and `not`",
     question="""Do the iRule word operators `and`, `or` and `not` bind exactly like `&&`, `||` and `!` (including mixed with the symbolic forms and with `?:`), and do `and`/`or` short-circuit? Each bare expression is followed by its two explicit groupings. Scope: TMM `expr` with braced literal expressions in `RULE_INIT`.""",
     body=rows_rule_init("oplogic", expr_rows(LOGIC)))

case(id="opdyn", slug="bigip-f5-word-operators-in-computed-expr",
     title="iRule word operators in a computed `expr` string",
     question="""Are the iRule word operators also recognised when the expression is a runtime string rather than a braced literal the rule loader can see, and does the loader comment on it? This decides whether the analyser may treat a computed `expr` string as iRules expression syntax.""",
     body=("when RULE_INIT {\n"
           + "".join(f"set s [binary format H* {e.encode().hex()}]\nset c [catch {{expr $s}} v]\n{L('opdyn', lbl, '$c', '$v')}\n"
                     for lbl, e in [("contains", '"ab" contains "a" == 1'), ("matches", '"abc" matches "a*"'),
                                    ("or_and", "1 or 0 and 0"), ("not", "not 2 == 1"),
                                    ("starts_with_plus", '"12" starts_with 1 + 1')])
           + "}\n"))

# ------------------------------------------------------------ signed values
case(id="sginit", slug="bigip-tmm-signed-scan-and-format",
     title="TMM `binary scan`, `scan` and `format` with signed values",
     question="""Does TMM's `binary scan`, `scan` and `format` treat signed values the way stock Tcl 8.4 does: signed byte and short fields, the `u` flag, `%x`/`%lx` of `ffffffff`, hex, unsigned and character formatting of -127, and the integer width that `<<` and `+` overflow at? This is the TMM half of the W156 question; F5 command arguments are separate cases.""",
     body=rows_rule_init("sginit", [
         ("bscan_cS", r"binary scan \x81\x00\x80 cS b s; list $b $s"),
         ("bscan_cs_le", r"binary scan \x81\x00\x80 cs b s; list $b $s"),
         ("bscan_S_8000", r"binary scan \x80\x00 S s; set s"),
         ("bscan_masked", r"binary scan \x81\x00\x80 cS b s; list [expr {$b & 0xff}] [expr {$s & 0xffff}]"),
         ("bscan_cu", r"binary scan \x81 cu b; set b"),
         ("scan_x", "scan ffffffff %x v; set v"),
         ("scan_lx", "scan ffffffff %lx v; set v"),
         ("fmt_hex", "format %02x -127"),
         ("fmt_hex_masked", "format %02x [expr {-127 & 0xff}]"),
         ("fmt_u", "format %u -127"),
         ("fmt_c_hex", "binary scan [format %c -127] H* h; set h"),
         ("fmt_c_len", "string length [format %c -127]"),
         ("intwidth", "list [expr {1 << 31}] [expr {1 << 32}] [expr {0x7fffffff + 1}] [expr {-1 >> 1}] [expr {1 << 63}] [expr {1 << 64}]"),
         ("ipaddr_ctl_eq", "IP::addr 192.192.255.1 equals 192.192.255.1"),
         ("ipaddr_ctl_mask", "IP::addr 192.192.255.1 mask 255.255.255.0"),
         ("ipaddr_dyn_eq", "set a 192.[expr {-64}].[expr {-1}].1; IP::addr $a equals 192.192.255.1"),
         ("ipaddr_dyn_mask", "set a 192.[expr {-64}].[expr {-1}].1; IP::addr $a mask 255.255.255.0"),
     ]))

case(id="sgipaddr", slug="bigip-ip-addr-literal-signed-octet-refused",
     title="`IP::addr` with a literal negative octet at rule load",
     question="""Does the rule loader accept `IP::addr` given a literal dotted quad with negative octets, as a signed `binary scan` value formatted with `%d` would produce? The runtime behaviour with a computed address is in the TMM signed-value case; this one records the load-time verdict only.""",
     body="when RULE_INIT {\nset r1 [IP::addr 192.-64.-1.1 equals 192.192.255.1]\nset r2 [IP::addr 192.-64.-1.1 mask 255.255.255.0]\n}\n")

case(id="sgport", slug="bigip-node-and-pool-member-negative-port-refused",
     title="`node` and `pool ... member` with a negative port at rule load",
     question="""Does the rule loader accept a literal negative port for `node` and `pool <pool> member`? A negative port is what a signed 16-bit scan of a port above 32767 yields.""",
     extra=f"ltm pool tcllsp_oq_sgport_pool {{ members {{ 10.0.0.1:80 {{ }} }} }}\n",
     extra_del=["ltm pool tcllsp_oq_sgport_pool"],
     body="when HTTP_REQUEST {\nnode 10.0.0.1 -1\npool tcllsp_oq_sgport_pool member 10.0.0.1 -1\n}\n")

case(id="sgtable", slug="bigip-table-incr-negative",
     title="`table incr` with a negative increment",
     question="""What does `table incr` store and return for a negative increment, compared with its positive twin? A counter fed a signed scanned value would take this path.""",
     http=True,
     body=("when HTTP_REQUEST {\n"
           "set c [catch {table set tcllsp_oq_sgtable_k 0 indef indef; set r [table incr tcllsp_oq_sgtable_k -32768]; lappend r [table lookup tcllsp_oq_sgtable_k]; table delete tcllsp_oq_sgtable_k; set r} v]\n"
           + L("sgtable", "neg", "$c", "$v") + "\n"
           "set c [catch {table set tcllsp_oq_sgtable_k 0 indef indef; set r [table incr tcllsp_oq_sgtable_k 32768]; lappend r [table lookup tcllsp_oq_sgtable_k]; table delete tcllsp_oq_sgtable_k; set r} v]\n"
           + L("sgtable", "pos", "$c", "$v") + "\n"
           "set c [catch {table set tcllsp_oq_sgtable_k 5 indef indef; set r [table incr tcllsp_oq_sgtable_k -1]; table delete tcllsp_oq_sgtable_k; set r} v]\n"
           + L("sgtable", "dec_from_5", "$c", "$v") + "\n"
           'HTTP::respond 200 content "ok\\n" Connection close\n}\n'))

case(id="sgclass", slug="bigip-class-match-signed-key",
     title="Integer data-group lookups with a signed key",
     question="""Does an integer data group holding 129 match the signed form -127 of the same byte, and how do `class match` and `class lookup` parse a leading negative key?""",
     http=True,
     extra="ltm data-group internal tcllsp_oq_sgclass_dg { type integer records { 129 { data unsigned } } }\n",
     extra_del=["ltm data-group internal tcllsp_oq_sgclass_dg"],
     body=("when HTTP_REQUEST {\n"
           + "".join(f"set c [catch {{{s}}} v]\n{L('sgclass', lbl, '$c', '$v')}\n" for lbl, s in [
               ("match_neg", "class match -127 equals tcllsp_oq_sgclass_dg"),
               ("match_dd_neg", "class match -- -127 equals tcllsp_oq_sgclass_dg"),
               ("match_dd_pos", "class match -- 129 equals tcllsp_oq_sgclass_dg"),
               ("match_dd_masked", "class match -- [expr {-127 & 0xff}] equals tcllsp_oq_sgclass_dg"),
               ("lookup_neg", "class lookup -127 tcllsp_oq_sgclass_dg"),
               ("lookup_pos", "class lookup 129 tcllsp_oq_sgclass_dg"),
           ])
           + 'HTTP::respond 200 content "ok\\n" Connection close\n}\n'))

case(id="sghttp", slug="bigip-http-collect-and-payload-negative",
     title="`HTTP::collect` and `HTTP::payload` with -1",
     question="""What do `HTTP::collect -1` and `HTTP::payload -1` do at load and at runtime on a request carrying an 8-byte body?""",
     http=True, post=True,
     body=("when HTTP_REQUEST {\n"
           "set c [catch {HTTP::collect -1} v]\n" + L("sghttp", "collect", "$c", "$v") + "\n"
           "set c [catch {string length [HTTP::payload -1]} v]\n" + L("sghttp", "payload", "$c", "$v") + "\n"
           'HTTP::respond 200 content "ok\\n" Connection close\n}\n'))

case(id="sgtcpcollect", slug="bigip-tcp-collect-negative",
     title="`TCP::collect -1`",
     question="""What does `TCP::collect -1` do at load and at runtime: does the client data event fire, and with how many bytes, when the client sends 8 bytes?""",
     tcp=True,
     body=("when CLIENT_ACCEPTED {\nset c [catch {TCP::collect -1} v]\n" + L("sgtcpcollect", "CLIENT_ACCEPTED", "$c", "$v") + "\n}\n"
           "when CLIENT_DATA {\n" + L("sgtcpcollect", "CLIENT_DATA", "[TCP::payload length]") + "\nTCP::respond \"seen\\n\"\nTCP::close\n}\n"))

case(id="sgtcppayload", slug="bigip-tcp-payload-release-negative",
     title="`TCP::payload` offsets and lengths and `TCP::release` with -1",
     question="""With 8 collected bytes, what do `TCP::payload -1 4`, `TCP::payload 0 -1` and `TCP::release -1` do at load and at runtime, compared with `TCP::payload 0 4`?""",
     tcp=True,
     body=("when CLIENT_ACCEPTED {\nTCP::collect 8\n}\n"
           "when CLIENT_DATA {\n"
           + "".join(f"set c [catch {{{s}}} v]\n{L('sgtcppayload', lbl, '$c', '$v')}\n" for lbl, s in [
               ("ctl_0_4", "string length [TCP::payload 0 4]"),
               ("off_neg", "string length [TCP::payload -1 4]"),
               ("len_neg", "string length [TCP::payload 0 -1]"),
               ("release_neg", "TCP::release -1"),
           ])
           + L("sgtcppayload", "after_release", "[TCP::payload length]") + "\nTCP::respond \"done\\n\"\nTCP::close\n}\n"))

case(id="sgtcpreplace", slug="bigip-tcp-payload-replace-negative-refused",
     title="`TCP::payload replace` with a negative offset at rule load",
     question="""Does the rule loader accept `TCP::payload replace -1 1 x`?""",
     body="when CLIENT_DATA {\nTCP::payload replace -1 1 x\n}\n")

case(id="sgudp", slug="bigip-udp-payload-negative",
     title="`UDP::payload` offsets and lengths with -1",
     question="""With an 8-byte datagram, what do `UDP::payload -1 4` and `UDP::payload 0 -1` do at load and at runtime, compared with `UDP::payload 0 4`?""",
     udp=True,
     body=("when CLIENT_DATA {\n"
           + "".join(f"set c [catch {{{s}}} v]\n{L('sgudp', lbl, '$c', '$v')}\n" for lbl, s in [
               ("ctl_0_4", "string length [UDP::payload 0 4]"),
               ("off_neg", "string length [UDP::payload -1 4]"),
               ("len_neg", "string length [UDP::payload 0 -1]"),
           ])
           + "UDP::drop\n}\n"))

# ------------------------------------------------------- RULE_INIT flow
case(id="riflow", slug="bigip-rule-init-flow-commands-runtime",
     title="Flow commands executed in `RULE_INIT`",
     question="""The 2026-08-26 matrix recorded `HTTP::uri`, `HTTP::status`, `HTTP::respond`, `HTTP::collect`, `TCP::collect` and `TCP::payload` as accepted in `RULE_INIT` at load. What does the loader say about them now, and what happens when each actually runs there: error, empty result, or effect? Does the handler continue after a caught failure?""",
     body=rows_rule_init("riflow", [
         ("HTTP::uri", "HTTP::uri"), ("HTTP::status", "HTTP::status"),
         ("HTTP::respond", 'HTTP::respond 200 content "x"'), ("HTTP::collect", "HTTP::collect 1"),
         ("TCP::collect", "TCP::collect 1"), ("TCP::payload", "TCP::payload"),
     ]).replace("}\n", L("riflow", "reached-end") + "\n}\n"))

# ---------------------------------------------------------------- switch
SW_LISTS = [("adjacent", "a{set r A}"), ("quoted", '"a"{set r A}'), ("braced", "{a}{set r A}"),
            ("braced_default", "{x} {set r X} {default}{set r D}"), ("ctl", "a {set r A}")]
case(id="swdyn", slug="bigip-switch-body-list-runtime",
     title="Is a `switch` pattern/body list split with the iRule word break at runtime?",
     question="""When the `switch` pattern/body list is a runtime value (so the rule loader never sees it), does TMM split it with stock Tcl list rules or with the iRule implicit word break after a closing brace or quote (R1/R2)? `llength` of the same value is the list-parser control.""",
     body=("when RULE_INIT {\n"
           + "".join(f"set b [binary format H* {v.encode().hex()}]\nset r unset\nset l [catch {{llength $b}} n]\nset c [catch {{switch a $b}} v]\n"
                     f"{L('swdyn', lbl, 'llength=$l:$n', 'switch=$c:$v', 'r=$r')}\n" for lbl, v in SW_LISTS)
           + "}\n"))
case(id="swlit", slug="bigip-switch-body-list-at-load",
     title="Literal `switch` pattern/body lists that need the iRule word break",
     question="""Does the rule loader parse a literal braced `switch` pattern/body list with stock list rules or with the iRule implicit word break? Each list here is valid only if the word break applies.""",
     body=("when RULE_INIT {\n"
           + "".join(f"switch a {{{v}}}\n" for lbl, v in SW_LISTS if lbl in ("quoted", "braced", "braced_default"))
           + "}\n"))
case(id="swargs", slug="bigip-switch-separate-args-word-break",
     title="`switch` patterns and bodies as separate words with the iRule word break",
     question="""When the same patterns and bodies are separate `switch` words rather than one list, does the script-level implicit word break (R1/R2) split `"a"{...}`, `{a}{...}` and `{default}{...}` into pattern and body?""",
     body=("when RULE_INIT {\n"
           + "".join(f"set r unset\nset c [catch {{switch a {v}}} v]\n{L('swargs', lbl, '$c', '$v', 'r=$r')}\n"
                     for lbl, v in SW_LISTS if lbl in ("quoted", "braced", "braced_default", "ctl"))
           + "}\n"))

# -------------------------------------------------------------------- N1
case(id="n1event", slug="bigip-brace-line-continuation-event-time-priority",
     title="Brace-line continuation (N1) at event time in a prioritised rule",
     question="""Does the brace-line continuation (N1) still hold when the code runs at event time on a live request, inside a rule with a rule-level `priority` and a `when ... priority` handler, and inside a proc called from it?""",
     http=True,
     body=("priority 100\n"
           "proc tcllsp_oq_p {a}\n{\nreturn \"p-$a\"\n}\n"
           "when HTTP_REQUEST priority 900 {\n"
           "set n 0\n"
           "while {$n < 30}\n{\nset n [expr {$n + 30}]\n}\n"
           "set l [list a b\n{c}]\n"
           "set p [call tcllsp_oq_p x]\n"
           + L("n1event", "HTTP_REQUEST", "n=$n", "l=$l", "p=$p") + "\n"
           'HTTP::respond 200 content "ok\\n" Connection close\n}\n'))
case(id="n1prio", slug="bigip-brace-line-continuation-priority-words",
     title="Brace-line continuation (N1) in `priority` and `when` header words",
     question="""Does N1 also apply to the rule-file structure: a rule-level `priority` whose value is on the next line in braces, and a `when EVENT priority` whose priority value and body start on following lines?""",
     body=("priority\n{200}\n"
           "when RULE_INIT priority\n{10}\n{\n" + L("n1prio", "RULE_INIT", "ran") + "\n}\n"))


def virtual(cid, port, profiles, proto="tcp"):
    return (f"ltm virtual tcllsp_oq_{cid}_vs {{ destination {VIP}:{port} mask 255.255.255.255 "
            f"ip-protocol {proto} profiles {{ {profiles} }} source-address-translation {{ type automap }} "
            f"rules {{ tcllsp_oq_{cid} }} }}\n")


def gen(outdir):
    out = pathlib.Path(outdir)
    out.mkdir(parents=True, exist_ok=True)
    port = 19700
    for c in CASES:
        cid = c["id"]
        port += 1
        conf = c.get("extra", "") + f"ltm rule tcllsp_oq_{cid} {{\n{c['body']}}}\n"
        dels = [f"ltm rule tcllsp_oq_{cid}"] + c.get("extra_del", [])
        traffic = None
        if c.get("http"):
            conf += virtual(cid, port, "tcp { } http { }")
            data = "-d abcdefgh " if c.get("post") else ""
            traffic = f"curl -s -m 5 {data}-w ' http=%{{http_code}} exit=' http://{VIP}:{port}/ ; echo $?\n"
        elif c.get("tcp"):
            conf += virtual(cid, port, "tcp { }")
            traffic = f"printf abcdefgh | nc -w 5 {VIP} {port}; echo \" exit=$?\"\n"
        elif c.get("udp"):
            conf += virtual(cid, port, "udp { }", proto="udp")
            traffic = f"printf abcdefgh | nc -u -w 1 {VIP} {port}; echo \" exit=$?\"\n"
        if traffic:
            dels = [f"ltm virtual tcllsp_oq_{cid}_vs"] + dels
            c["traffic"] = traffic
        c["conf"] = conf
        (out / f"{cid}.conf").write_text(conf)
        (out / f"{cid}.delete").write_text("".join(f"tmsh delete {d}\n" for d in dels))
        if traffic:
            (out / f"{cid}.traffic").write_text(traffic)
    print(len(CASES), "cases")


def render(drive_out, evdir):
    text = open(drive_out, errors="replace").read().splitlines()
    by = {}
    for line in text:
        m = re.match(r"^(RESULT|TRAFFIC|OTHER|LEFTOVER)~~([a-z0-9]+)~~(.*)$", line)
        if m:
            by.setdefault(m.group(2), []).append((m.group(1), m.group(3)))
            continue
        m = re.match(r"^(\d+) LOG~~([a-z0-9]+)\|(.*)$", line)
        if m:
            by.setdefault(m.group(2), []).append(("LOG", f"{m.group(1)}x {m.group(3)}"))
    for c in CASES:
        cid = c["id"]
        recs = by.get(cid, [])
        verdict = next((v for k, v in recs if k == "RESULT"), "not recorded")
        status, _, msg = verdict.partition("~~")
        logs = [v for k, v in recs if k == "LOG"]
        traffic = [v for k, v in recs if k == "TRAFFIC"]
        loader_msg = msg.strip() or "(none)"
        channel = ("`RULE_INIT`, run by every TMM when the rule loads" if "RULE_INIT" in c["body"] and not (c.get("http") or c.get("tcp") or c.get("udp"))
                   else "a request through a dedicated virtual server (SNAT automap) sent from a host on the same LAN")
        lines = [f"# {c['title']}", "", "## Question", "", textwrap.fill(c["question"], 10000), "",
                 "## Assumptions", "",
                 f"Run on {DATE} against {BUILD}. The script is the body of an `ltm rule` named `tcllsp_oq_{cid}`, "
                 "loaded with `tmsh load sys config merge file` after proving its object names absent, and deleted afterwards; "
                 "the configuration was never saved. "
                 + ("Supporting objects in the same file: " + c["extra"].strip().replace("\n", "; ") + ". " if c.get("extra") else "")
                 + f"Results are the loader's response and the tagged `/var/log/ltm` lines the rule wrote, collected through {channel}. "
                 "Syslog collapses identical lines, so the count beside each log line is the number of distinct TMM units that logged it, not an execution count. "
                 + ("Client: `" + c["traffic"].strip() + "` from the driver host. " if c.get("traffic") else "")
                 + "The per-cell probes in `scripts/dev/bigip-probes/open-questions/gen.py` ran each expression as its own rule first and every one loaded; this grouped rule is the recorded run.",
                 "", "## Script", "", "```tcl-visible", c["body"].rstrip("\n"), "```", "",
                 "## Results", "", f"### {BUILD.split(' (')[0]}, TMM", "",
                 "- Channel: rule load (mcpd) and TMM log lines.",
                 "- Exit status: Not recorded",
                 "- Standard output: Not recorded",
                 "- Standard error: Not recorded",
                 "- Recorded result:", "", "```text-visible",
                 f"LOAD {status}: {loader_msg}"]
        lines += [f"TRAFFIC {t.strip()}" for t in traffic]
        lines += [f"LOG {l}" for l in logs] or ["LOG (no tagged lines)"]
        lines += ["```", "",
                  "`LOAD` is the loader verdict (`ACCEPT`, `WARN` = loaded with warnings, `REJECT` = refused) and its message. "
                  "`LOG <n>x <label>|<catch code>|<result>` is one tagged log line with its `TCLLSP_OQ|<case>|` prefix removed; `TRAFFIC` is the client's own output.", ""]
        (pathlib.Path(evdir) / f"{c['slug']}.md").write_text("\n".join(lines))
    print("rendered", len(CASES))


if __name__ == "__main__":
    if sys.argv[1] == "gen":
        gen(sys.argv[2])
    else:
        gen(sys.argv[4])  # rebuild case metadata (conf, traffic) without touching the appliance
        render(sys.argv[2], sys.argv[3])
