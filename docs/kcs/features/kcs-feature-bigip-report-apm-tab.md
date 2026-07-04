# KCS: feature — BIG-IP report APM access-profile tab

> **Audience:** User
> **Type:** Functionality

## Summary

An APM tab in the standalone BIG-IP HTML report that walks each
`apm profile access` out to every object it depends on and draws it as a
Visual-Policy-Editor-style dependency graph.

## Applies to

tcl-lsp CLI

## Question

What does the APM tab show, and how do I use it?

## How to use

Generate a report from a config export that contains APM objects (a per-session
access policy, its profile, agents and resources). When the device has at least
one `apm profile access`, an **APM** tab appears in the report's tab strip with
a count of access profiles.

Each access profile is drawn as one graph, laid out left-to-right like the F5
Visual Policy Editor (the drag-and-drop access-policy builder in the BIG-IP
GUI):

- the **virtual servers** that attach the profile, and the profile's
  **connectivity profile** (a sibling profile on the same virtual);
- the **access policy** and its **items**, with the `next-item` flow between
  them labelled by each branch caption (`Successful`, `fallback`, …);
- the **agents** on each item — the AAA server an auth agent authenticates
  against, and the resources a resource-assign agent hands out
  (network-access → lease pool and client DNS, webtops, remote-desktop and
  portal-access resources).

Boxes are rectangular with true orthogonal (right-angle) connectors — the
graph is laid out by [elkjs](https://github.com/kieler/elkjs), which routes the
edges into separate channels with each arrowhead seated on the node border. The
**Start** and **Allow** endings are green and **Deny** is red, matching the
editor.

## Example

For a `mycave` access profile whose policy is Logon Page → AD Auth → Full
Resource Assign → Allow / Deny, the tab renders:

```
Virtual Server mycave_vs ─▶ Access Profile mycave ─▶ Access Policy mycave
   └▶ Connectivity mycave_cp                            └▶ Start ─▶ Logon Page
AD Auth ─Successful▶ Full Resource Assign ─fallback▶ Allow
AD Auth ─fallback▶ Deny            AD Auth ─auth▶ AAA Server mycave_aaa_srvr
Full Resource Assign ─network access▶ Network Access mycave_na_res
   └▶ Lease Pool mycave_lp   └▶ Client DNS 192.168.1.6
Full Resource Assign ─webtop▶ Webtop mycave_rdp
```

## Why it is built this way

The `f5-query` DSL projection only covers the `ltm` module, and the parsed
model keeps APM objects as *minimal* records that drop the linking fields the
walk needs — the profile's `access-policy` pointer, an item's `next-item`
edges, and a resource-assign agent's assigned resources. So the APM walk reads
the `apm …` stanzas straight from the config text, the same file-first approach
the SSL-certificate and secrets tabs take for their non-LTM data.
