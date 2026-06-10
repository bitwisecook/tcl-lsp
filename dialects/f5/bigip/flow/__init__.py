"""Pcap/flow extraction layer for ``f5 explain-flow``.

Config-agnostic packet decoding, flow/session pairing, and tshark
enrichment.  The BigIP-config-aware orchestration, virtual-server
matching, policy evaluation, and report rendering stay in the parent
``explain_flow`` module, which re-exports the public names.
"""
