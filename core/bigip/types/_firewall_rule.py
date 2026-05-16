"""Typed BIG-IP firewall + NAT rule values.

``security firewall rule-list`` / ``security firewall policy`` /
``security nat policy`` each carry a ``rules { ... }`` block whose
items are keyed sub-blocks with structured source / destination /
action clauses:

    rules {
        allow_https {
            action accept
            ip-protocol tcp
            destination {
                port-lists { /Common/_sys_self_allow_tcp_defaults }
                ports { 443 }
                address-lists { /Common/web_servers }
            }
            source {
                address-lists { /Common/trusted_clients }
                ports { 1024-65535 }
            }
        }
    }

The legacy parser captures the rule body as raw text; this typed
value surfaces the structure so safety queries can ask "which rules
are any-any-accept?" / "which rules reference port-list X?" / "what
addresses can reach virtual Y on port 443?" without re-parsing the
nested blocks each time.
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class FirewallEndpoint:
    """One side (source or destination) of a firewall rule.

    Captures the lists of port-list refs, port literals + ranges,
    and address-list refs.  Each list keeps the original ordering
    so a round trip through the rewrite layer preserves the user's
    intent.
    """

    port_lists: tuple[str, ...] = ()
    ports: tuple[str, ...] = ()
    address_lists: tuple[str, ...] = ()
    addresses: tuple[str, ...] = ()

    @classmethod
    def from_raw(cls, body: str) -> "FirewallEndpoint":
        port_lists: list[str] = []
        ports: list[str] = []
        address_lists: list[str] = []
        addresses: list[str] = []
        tokens = body.split()
        idx = 0
        while idx < len(tokens):
            tok = tokens[idx]
            if tok in ("port-lists", "ports", "address-lists", "addresses") and (
                idx + 1 < len(tokens) and tokens[idx + 1] == "{"
            ):
                # Walk to the matching brace.
                target = {
                    "port-lists": port_lists,
                    "ports": ports,
                    "address-lists": address_lists,
                    "addresses": addresses,
                }[tok]
                idx += 2
                while idx < len(tokens) and tokens[idx] != "}":
                    target.append(tokens[idx])
                    idx += 1
                if idx < len(tokens):
                    idx += 1
                continue
            idx += 1
        return cls(
            port_lists=tuple(port_lists),
            ports=tuple(ports),
            address_lists=tuple(address_lists),
            addresses=tuple(addresses),
        )

    def references(self) -> tuple[tuple[str, str], ...]:
        """Yield ``(kind, full-path)`` pairs for every list reference."""
        out: list[tuple[str, str]] = []
        for path in self.port_lists:
            out.append(("security firewall port-list", path))
        for path in self.address_lists:
            out.append(("security firewall address-list", path))
        return tuple(out)


@dataclass(frozen=True, slots=True)
class FirewallRule:
    """One rule inside a firewall rule-list / policy.

    The classical 5-tuple is exposed as a structured value:

    - ``name`` — the rule's bare name (the key in the parent
      ``rules { ... }`` keyed-block list).
    - ``action`` — ``accept`` / ``drop`` / ``reject`` /
      ``accept-decisively`` / ``log-only``.
    - ``ip_protocol`` — TMSH-spelt protocol (``tcp`` / ``udp`` /
      ``icmp`` / ``any``).
    - ``source`` / ``destination`` — :class:`FirewallEndpoint`
      values capturing each side's port + address constraints.
    - ``log`` — whether the rule logs matches.
    - ``rule_list`` — when the rule is a *list-reference* (``rule-list
      <full-path>``) instead of an inline classifier, this carries
      the path; ``action`` / ``ip_protocol`` / ``source`` /
      ``destination`` will be empty.

    ``raw`` preserves the original body for round trips and for the
    less-common variants this typed value doesn't surface
    explicitly.
    """

    name: str = ""
    action: str = ""
    ip_protocol: str = ""
    log: bool = False
    source: FirewallEndpoint = FirewallEndpoint()
    destination: FirewallEndpoint = FirewallEndpoint()
    rule_list: str = ""
    raw: str = ""

    @classmethod
    def from_raw(cls, name: str, body: str) -> "FirewallRule":
        action = ""
        ip_protocol = ""
        log = False
        rule_list = ""
        source = FirewallEndpoint()
        destination = FirewallEndpoint()

        # Walk to find action / ip-protocol / log / rule-list /
        # source { ... } / destination { ... }.  Sub-block bodies
        # are split out with a brace-balanced sub-walk so nested
        # braces inside source/destination don't terminate the
        # outer scan prematurely.
        tokens, sub_bodies = _split_sub_blocks(body)
        idx = 0
        while idx < len(tokens):
            tok = tokens[idx]
            if tok == "action" and idx + 1 < len(tokens):
                action = tokens[idx + 1]
                idx += 2
                continue
            if tok == "ip-protocol" and idx + 1 < len(tokens):
                ip_protocol = tokens[idx + 1]
                idx += 2
                continue
            if tok == "log":
                log = True
                idx += 1
                continue
            if tok == "rule-list" and idx + 1 < len(tokens):
                rule_list = tokens[idx + 1]
                idx += 2
                continue
            idx += 1
        if "source" in sub_bodies:
            source = FirewallEndpoint.from_raw(sub_bodies["source"])
        if "destination" in sub_bodies:
            destination = FirewallEndpoint.from_raw(sub_bodies["destination"])
        return cls(
            name=name,
            action=action,
            ip_protocol=ip_protocol,
            log=log,
            source=source,
            destination=destination,
            rule_list=rule_list,
            raw=body.strip(),
        )

    def references(self) -> tuple[tuple[str, str], ...]:
        """Yield every outbound reference the rule carries.

        Includes any port-list / address-list refs on either
        endpoint plus the rule-list reference when this is a
        list-shaped rule.
        """
        out: list[tuple[str, str]] = []
        out.extend(self.source.references())
        out.extend(self.destination.references())
        if self.rule_list:
            out.append(("security firewall rule-list", self.rule_list))
        return tuple(out)


def _split_sub_blocks(body: str) -> tuple[list[str], dict[str, str]]:
    """Tokenise *body* into top-level tokens + a ``{name: sub-body}``
    map of sub-block contents.

    Handles brace nesting so ``source { addresses { 10.0.0.0/8 } }``
    captures the entire ``addresses { 10.0.0.0/8 }`` inside the
    source sub-block rather than terminating the source body at the
    first inner ``}``.
    """
    sub_bodies: dict[str, str] = {}
    tokens: list[str] = []
    raw_tokens = body.split()
    idx = 0
    while idx < len(raw_tokens):
        tok = raw_tokens[idx]
        if (
            tok in ("source", "destination")
            and idx + 1 < len(raw_tokens)
            and raw_tokens[idx + 1] == "{"
        ):
            depth = 1
            idx += 2
            sub_parts: list[str] = []
            while idx < len(raw_tokens) and depth > 0:
                t = raw_tokens[idx]
                if t == "{":
                    depth += 1
                elif t == "}":
                    depth -= 1
                    if depth == 0:
                        break
                sub_parts.append(t)
                idx += 1
            if idx < len(raw_tokens):
                idx += 1  # skip the closing ``}``
            sub_bodies[tok] = " ".join(sub_parts)
            continue
        tokens.append(tok)
        idx += 1
    return tokens, sub_bodies


# NAT rules share the firewall rule shape — same source /
# destination endpoints, same action vocabulary — so we model them
# as a thin alias.  The dispatch can still split them at the spec
# layer for kind-specific behaviour later.
NatRule = FirewallRule


__all__ = ["FirewallEndpoint", "FirewallRule", "NatRule"]
