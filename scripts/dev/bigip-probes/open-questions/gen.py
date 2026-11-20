#!/usr/bin/env python3
"""Materialise the open-question probes into per-case tmsh configs.

Usage: gen.py <outdir>

Writes, per case, <id>.conf (loadable with `tmsh load sys config merge file`),
<id>.delete (exact-name tmsh deletes, virtual servers first) and, for traffic
cases, <id>.traffic (bash run on the appliance). Every object is named
tcllsp_oq_<id>[...]; every log line carries TCLLSP_OQ|<id>|.

Suites:
  op_*  F5 word operators: `matches` semantics and binding power (RULE_INIT)
  sg_*  signed integers reaching F5 and stock commands (RULE_INIT or traffic)
  ri_*  the six RULE_INIT compile acceptances, executed (RULE_INIT)
  sw_*  whether the `switch` body is re-parsed with the implicit word break
  n1_*  brace-line continuation (N1) under `priority` and event-time execution
"""
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
OUT = pathlib.Path(sys.argv[1])
OUT.mkdir(parents=True, exist_ok=True)

VIP = "192.168.9.24"  # single-NIC appliance: virtuals share the self address
PORT_BASE = 19300


def write(cid, conf, deletes, traffic=None):
    (OUT / f"{cid}.conf").write_text(conf)
    (OUT / f"{cid}.delete").write_text("".join(f"tmsh delete {d}\n" for d in deletes))
    if traffic:
        (OUT / f"{cid}.traffic").write_text(traffic)


def rule(cid, body):
    return f"ltm rule tcllsp_oq_{cid} {{\n{body}}}\n"


def log(cid, *fields):
    return 'log local0. "TCLLSP_OQ|' + cid + "|" + "|".join(fields) + '"\n'


# ---------------------------------------------------------------- operators
def operators():
    for line in (HERE / "operators.cases").read_text().splitlines():
        if not line or line.startswith("#"):
            continue
        name, expr = line.split("\t", 1)
        cid = f"op_{name}"
        hexed = expr.encode().hex()
        body = (
            "when RULE_INIT {\n"
            f"set tcllsp_oq_c [catch {{expr {{{expr}}}}} tcllsp_oq_v]\n"
            f"set tcllsp_oq_s [binary format H* {hexed}]\n"
            "set tcllsp_oq_d [catch {expr $tcllsp_oq_s} tcllsp_oq_w]\n"
            + log(cid, "lit", "$tcllsp_oq_c", "$tcllsp_oq_v", "dyn", "$tcllsp_oq_d", "$tcllsp_oq_w")
            + "}\n"
        )
        write(cid, rule(cid, body), [f"ltm rule tcllsp_oq_{cid}"])


# ------------------------------------------------------------ signed values
# RULE_INIT cases: (id, script). Each runs under catch; result logged.
SIGNED_INIT = [
    ("bscan", r"binary scan \x81\x00\x80 cS b s; list $b $s"),
    ("bscan_cu", r"binary scan \x81 cu b; set b"),
    ("bscan_masked", r"binary scan \x81\x00\x80 cS b s; list [expr {$b & 0xff}] [expr {$s & 0xffff}]"),
    ("scan_x", "scan ffffffff %x v; set v"),
    ("scan_lx", "scan ffffffff %lx v; set v"),
    ("fmt_hex", "format %02x -127"),
    ("fmt_hex_masked", "format %02x [expr {-127 & 0xff}]"),
    ("fmt_u", "format %u -127"),
    ("fmt_c", "binary scan [format %c -127] H* h; set h"),
    ("fmt_c_len", "string length [format %c -127]"),
    ("intwidth", "list [expr {1 << 31}] [expr {1 << 32}] [expr {0x7fffffff + 1}] [expr {-1 >> 1}]"),
    ("ipaddr_eq", "IP::addr 192.-64.-1.1 equals 192.192.255.1"),
    ("ipaddr_mask", "IP::addr 192.-64.-1.1 mask 255.255.255.0"),
    ("ipaddr_ctl_eq", "IP::addr 192.192.255.1 equals 192.192.255.1"),
    ("ipaddr_ctl_mask", "IP::addr 192.192.255.1 mask 255.255.255.0"),
]

# Traffic cases on an HTTP virtual: (id, event, script)
SIGNED_HTTP = [
    ("table_incr", "HTTP_REQUEST",
     "table set tcllsp_oq_k 0 indef indef; set r [table incr tcllsp_oq_k -32768]; "
     "lappend r [table lookup tcllsp_oq_k]; table delete tcllsp_oq_k; set r"),
    ("table_incr_u", "HTTP_REQUEST",
     "table set tcllsp_oq_k 0 indef indef; set r [table incr tcllsp_oq_k 32768]; "
     "table delete tcllsp_oq_k; set r"),
    ("class_neg", "HTTP_REQUEST",
     "list [class match -127 equals tcllsp_oq_sg_class_neg_dg] [class match 129 equals tcllsp_oq_sg_class_neg_dg] "
     "[class lookup -127 tcllsp_oq_sg_class_neg_dg] [class lookup 129 tcllsp_oq_sg_class_neg_dg] "
     "[class match [expr {-127 & 0xff}] equals tcllsp_oq_sg_class_neg_dg]"),
    ("class_neg_dd", "HTTP_REQUEST",
     "list [class match -- -127 equals tcllsp_oq_sg_class_neg_dd_dg] [class match -- 129 equals tcllsp_oq_sg_class_neg_dd_dg] "
     "[class lookup -127 tcllsp_oq_sg_class_neg_dd_dg] [class lookup 129 tcllsp_oq_sg_class_neg_dd_dg] "
     "[class match -- [expr {-127 & 0xff}] equals tcllsp_oq_sg_class_neg_dd_dg]"),
    ("http_collect_neg", "HTTP_REQUEST", "HTTP::collect -1"),
    ("http_payload_neg", "HTTP_REQUEST", "string length [HTTP::payload -1]"),
    ("node_neg", "HTTP_REQUEST", "node 10.0.0.1 -1"),
    ("node_ctl", "HTTP_REQUEST", "node 10.0.0.1 80"),
    ("poolmember_neg", "HTTP_REQUEST", "pool tcllsp_oq_sg_poolmember_neg_pool member 10.0.0.1 -1"),
    ("poolmember_ctl", "HTTP_REQUEST", "pool tcllsp_oq_sg_poolmember_ctl_pool member 10.0.0.1 80"),
]

# Raw-TCP cases evaluated in CLIENT_DATA after `TCP::collect`.
SIGNED_TCP = [
    ("tcp_collect_neg", "CLIENT_ACCEPTED", "TCP::collect -1"),
    ("tcp_release_neg", "CLIENT_DATA", "TCP::release -1"),
    ("tcp_payload_off_neg", "CLIENT_DATA", "string length [TCP::payload -1 4]"),
    ("tcp_payload_len_neg", "CLIENT_DATA", "string length [TCP::payload 0 -1]"),
    ("tcp_payload_ctl", "CLIENT_DATA", "string length [TCP::payload 0 4]"),
    ("tcp_payload_repl_neg", "CLIENT_DATA", "TCP::payload replace -1 1 x; string length [TCP::payload]"),
]

SIGNED_UDP = [
    ("udp_payload_off_neg", "string length [UDP::payload -1 4]"),
    ("udp_payload_len_neg", "string length [UDP::payload 0 -1]"),
    ("udp_payload_ctl", "string length [UDP::payload 0 4]"),
]


def catch_and_log(cid, script, where):
    return (
        f"set tcllsp_oq_c [catch {{{script}}} tcllsp_oq_v]\n"
        + log(cid, where, "$tcllsp_oq_c", "$tcllsp_oq_v")
    )


def signed():
    for name, script in SIGNED_INIT:
        cid = f"sg_{name}"
        body = "when RULE_INIT {\n" + catch_and_log(cid, script, "RULE_INIT") + "}\n"
        write(cid, rule(cid, body), [f"ltm rule tcllsp_oq_{cid}"])

    port = PORT_BASE
    for name, event, script in SIGNED_HTTP:
        cid = f"sg_{name}"
        port += 1
        extra, extra_del = "", []
        if name.startswith("class_neg"):
            extra = (f"ltm data-group internal tcllsp_oq_{cid}_dg {{ type integer records "
                     "{ 129 { data unsigned } } }\n")
            extra_del = [f"ltm data-group internal tcllsp_oq_{cid}_dg"]
        if name.startswith("poolmember"):
            extra = f"ltm pool tcllsp_oq_{cid}_pool {{ members {{ 10.0.0.1:80 {{ }} }} }}\n"
            extra_del = [f"ltm pool tcllsp_oq_{cid}_pool"]
        lb = ""
        if name.startswith(("node", "poolmember")):
            # HTTP::respond is rejected in LB_SELECTED; the client sees the
            # unreachable member instead, which is fine: the log is the result.
            lb = "when LB_SELECTED {\n" + log(cid, "LB_SELECTED", "[LB::server]") + "}\n"
            respond = ""
        else:
            respond = 'HTTP::respond 200 content "ok\\n" Connection close\n'
        body = (f"when {event} {{\n" + catch_and_log(cid, script, event) + respond + "}\n" + lb)
        if name.startswith("http_"):
            body += ("when HTTP_REQUEST_DATA {\n" + log(cid, "HTTP_REQUEST_DATA", "[HTTP::payload length]")
                     + 'HTTP::respond 200 content "data\\n" Connection close\n}\n')
        conf = extra + rule(cid, body) + virtual(cid, port, "tcp { } http { }")
        traffic = (f"curl -s -m 5 -o - -w ' http=%{{http_code}} exit=' "
                   f"-d 'abcdefgh' http://{VIP}:{port}/ ; echo $?\n")
        write(cid, conf, [f"ltm virtual tcllsp_oq_{cid}_vs", f"ltm rule tcllsp_oq_{cid}"] + extra_del, traffic)

    for name, event, script in SIGNED_TCP:
        cid = f"sg_{name}"
        port += 1
        if event == "CLIENT_ACCEPTED":
            body = ("when CLIENT_ACCEPTED {\n" + catch_and_log(cid, script, event) + "}\n"
                    "when CLIENT_DATA {\n" + log(cid, "CLIENT_DATA", "[TCP::payload length]")
                    + 'TCP::respond "seen\\n"\nTCP::close\n}\n')
        else:
            body = ("when CLIENT_ACCEPTED {\nTCP::collect 8\n}\n"
                    "when CLIENT_DATA {\n" + catch_and_log(cid, script, event)
                    + log(cid, "after", "[TCP::payload length]")
                    + 'TCP::respond "done\\n"\nTCP::close\n}\n')
        conf = rule(cid, body) + virtual(cid, port, "tcp { }")
        traffic = (f"printf 'abcdefgh' | nc -w 5 {VIP} {port}; echo \" exit=$?\"\n")
        write(cid, conf, [f"ltm virtual tcllsp_oq_{cid}_vs", f"ltm rule tcllsp_oq_{cid}"], traffic)

    for name, script in SIGNED_UDP:
        cid = f"sg_{name}"
        port += 1
        body = ("when CLIENT_DATA {\n" + catch_and_log(cid, script, "CLIENT_DATA")
                + log(cid, "after", "[UDP::payload length]") + "UDP::drop\n}\n")
        conf = rule(cid, body) + virtual(cid, port, "udp { }", proto="udp")
        traffic = f"printf 'abcdefgh' | nc -u -w 1 {VIP} {port}; echo \" exit=$?\"\n"
        write(cid, conf, [f"ltm virtual tcllsp_oq_{cid}_vs", f"ltm rule tcllsp_oq_{cid}"], traffic)


def virtual(cid, port, profiles, proto="tcp"):
    return (f"ltm virtual tcllsp_oq_{cid}_vs {{ destination {VIP}:{port} mask 255.255.255.255 "
            f"ip-protocol {proto} profiles {{ {profiles} }} source-address-translation {{ type automap }} "
            f"rules {{ tcllsp_oq_{cid} }} }}\n")


# ------------------------------------------------- RULE_INIT flow commands
RULE_INIT_FLOW = [
    ("http_uri", "HTTP::uri"),
    ("http_status", "HTTP::status"),
    ("http_respond", 'HTTP::respond 200 content "x"'),
    ("http_collect", "HTTP::collect 1"),
    ("tcp_collect", "TCP::collect 1"),
    ("tcp_payload", "TCP::payload"),
]


def rule_init_flow():
    for name, script in RULE_INIT_FLOW:
        cid = f"ri_{name}"
        body = ("when RULE_INIT {\n"
                f"set tcllsp_oq_c [catch {{{script}}} tcllsp_oq_v]\n"
                "set tcllsp_oq_e {}\ncatch {set tcllsp_oq_e $::errorCode}\n"
                + log(cid, "RULE_INIT", "$tcllsp_oq_c", "$tcllsp_oq_v", "errorCode=$tcllsp_oq_e")
                + log(cid, "reached-after") + "}\n")
        write(cid, rule(cid, body), [f"ltm rule tcllsp_oq_{cid}"])


# ------------------------------------------------------------------ switch
# Is the switch *pattern/body list* split with the implicit word break (R2)?
# Each body is built at runtime and passed to switch as a value, so the
# config layer and the rule-load parser never see it; then the same list is
# given literally in a second rule so load-time and runtime can be compared.
SWITCH = [
    ("adjacent", "a{set r A}"),          # R2 would split into `a` `{set r A}`
    ("quoted", '"a"{set r A}'),
    ("ctl", "a {set r A}"),
    ("braced", "{a}{set r A}"),          # R1/R2 on a brace-first word
    ("braced_default", "{x} {set r X} {default}{set r D}"),
    ("default_adjacent", "x {set r X} default{set r D}"),
]


def switch_cases():
    for name, blist in SWITCH:
        hexed = blist.encode().hex()
        cid = f"sw_{name}_dyn"
        body = ("when RULE_INIT {\n"
                f"set tcllsp_oq_b [binary format H* {hexed}]\n"
                "set r unset\n"
                "set tcllsp_oq_l [catch {llength $tcllsp_oq_b} tcllsp_oq_n]\n"
                "set tcllsp_oq_c [catch {switch a $tcllsp_oq_b} tcllsp_oq_v]\n"
                + log(cid, "llength", "$tcllsp_oq_l", "$tcllsp_oq_n", "switch", "$tcllsp_oq_c", "$tcllsp_oq_v", "r=$r")
                + "}\n")
        write(cid, rule(cid, body), [f"ltm rule tcllsp_oq_{cid}"])
        cid = f"sw_{name}_lit"
        body = ("when RULE_INIT {\n"
                "set r unset\n"
                f"set tcllsp_oq_c [catch {{switch a {{{blist}}}}} tcllsp_oq_v]\n"
                f"set tcllsp_oq_l [catch {{llength {{{blist}}}}} tcllsp_oq_n]\n"
                + log(cid, "llength", "$tcllsp_oq_l", "$tcllsp_oq_n", "switch", "$tcllsp_oq_c", "$tcllsp_oq_v", "r=$r")
                + "}\n")
        write(cid, rule(cid, body), [f"ltm rule tcllsp_oq_{cid}"])
        cid = f"sw_{name}_args"
        # the same pairs as separate switch arguments rather than one list
        body = ("when RULE_INIT {\n"
                "set r unset\n"
                f"set tcllsp_oq_c [catch {{switch a {blist}}} tcllsp_oq_v]\n"
                + log(cid, "switch", "$tcllsp_oq_c", "$tcllsp_oq_v", "r=$r")
                + "}\n")
        write(cid, rule(cid, body), [f"ltm rule tcllsp_oq_{cid}"])


# --------------------------------------------------------- N1 x priority
def n1_cases():
    # N1 continuation inside a prioritised rule, executed at event time on a
    # live request, plus inside a proc called at event time.
    port = PORT_BASE + 200
    cid = "n1_prio"
    body = ("priority 100\n"
            "proc tcllsp_oq_p {a}\n{\nreturn \"p-$a\"\n}\n"
            "when HTTP_REQUEST priority 900 {\n"
            "set n 0\n"
            "while {$n < 30}\n{\nset n [expr {$n + 30}]\n}\n"
            "set l [list a b\n{c}]\n"
            "set p [call tcllsp_oq_p x]\n"
            + log(cid, "HTTP_REQUEST", "n=$n", "l=$l", "p=$p")
            + 'HTTP::respond 200 content "ok\\n" Connection close\n}\n')
    conf = rule(cid, body) + virtual(cid, port, "tcp { } http { }")
    traffic = f"curl -s -m 5 -w ' http=%{{http_code}} exit=' http://{VIP}:{port}/ ; echo $?\n"
    write(cid, conf, [f"ltm virtual tcllsp_oq_{cid}_vs", f"ltm rule tcllsp_oq_{cid}"], traffic)

    # `when EVENT priority N` with the body brace on the next line
    cid = "n1_when_prio_nextline"
    body = ("when RULE_INIT priority 10\n{\n" + log(cid, "RULE_INIT", "ran") + "}\n")
    write(cid, rule(cid, body), [f"ltm rule tcllsp_oq_{cid}"])

    # `priority` itself split from its value by N1
    cid = "n1_when_prio_split"
    body = ("when RULE_INIT priority\n{10}\n{\n" + log(cid, "RULE_INIT", "ran") + "}\n")
    write(cid, rule(cid, body), [f"ltm rule tcllsp_oq_{cid}"])

    # rule-level `priority` with value on the next line
    cid = "n1_rule_prio_split"
    body = ("priority\n{200}\nwhen RULE_INIT {\n" + log(cid, "RULE_INIT", "ran") + "}\n")
    write(cid, rule(cid, body), [f"ltm rule tcllsp_oq_{cid}"])


operators()
signed()
rule_init_flow()
switch_cases()
n1_cases()
print(len(list(OUT.glob("*.conf"))), "cases")
