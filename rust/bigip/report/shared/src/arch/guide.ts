// tcl-lsp — a language server and toolchain for Tcl
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// The embedded guide for the architecture / topology manifest DSL, shown by the
// architecture editor. Kept in sync with parse_manifest in
// rust/tcl-bigip-query/src/architecture.rs.

export const DSL_GUIDE = `# Architecture & topology manifest — a small Tcl script, one command per line.
# Comments start with '#'. Brace {a value} to keep whitespace as one word.

# Devices: role (gtm/ltm/afm) + tier (0 = front) + display label.
device edge.ucs -role ltm -tier 1 -label "Edge"
device core.ucs -role ltm -tier 2

# Explicit inter-device links (auto-detection from address overlap still runs).
link edge.ucs core.ucs -label internal

# Network zones — named sets of IP ranges (v4 and v6).
zone external -cidr 0.0.0.0/0
zone dmz      -cidr 192.0.2.0/24 -cidr 2001:db8:dmz::/48
zone internal -cidr 10.0.0.0/8 -cidr 172.16.0.0/12

# Device interfaces attached to a zone (+ address).
interface edge.ucs ext0 -zone external -address 203.0.113.10

# DNS zones (the .zone file is uploaded / supplied as a side-input).
dns-zone example.com -file example.com.zone -zone dmz

# Enrichment maps.
cidr-name   10.1.0.0/16 {Datacenter A}
service-map -file services.csv     ;# port,name overrides (default: F5 table)
nat-map     -file nat.csv           ;# source,dest[,source_cidr,dest_cidr]
`;
