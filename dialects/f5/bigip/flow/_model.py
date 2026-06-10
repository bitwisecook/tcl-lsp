"""Data model for pcap flow extraction (Flow / Connection / Session / report)."""

from __future__ import annotations

from dataclasses import dataclass, field

from ..policy_eval import PolicyDecision


@dataclass(slots=True)
class Flow:
    """One unique unidirectional L3/L4 flow extracted from a capture.

    Flows are keyed by exact 5-tuple ``(src_ip, src_port, dst_ip,
    dst_port, proto)`` — the two halves of a TCP connection occupy two
    flow entries that :func:`pair_connections` later joins into a single
    :class:`Connection`.  Each flow accumulates counts plus L7 hints
    observed on its direction (TLS ClientHello vs ServerHello, HTTP
    request line vs response status line) and any TCP RST + F5 reset
    cause information attached to the trailer.
    """

    src_ip: str
    src_port: int
    dst_ip: str
    dst_port: int
    proto: int  # IP protocol number (6 TCP, 17 UDP, 1 ICMP, 58 ICMPv6)
    packets: int = 0
    bytes_total: int = 0
    tcp_syn: bool = False
    tcp_synack: bool = False
    tcp_fin: bool = False
    tcp_rst: bool = False
    tcp_rst_count: int = 0
    tcp_rst_after_bytes: int = 0  # bytes seen on this side before the first RST
    tls_clienthello: bool = False
    tls_sni: str = ""
    tls_version: str = ""  # legacy version inside ClientHello
    tls_chosen_version: str = ""  # version negotiated (from tshark)
    tls_chosen_cipher: str = ""  # ciphersuite chosen (from tshark)
    tls_alpn: str = ""  # ALPN protocol selected
    tls_alert_seen: bool = False
    tls_alert_desc: str = ""
    http_request_seen: bool = False
    http_method: str = ""
    http_host: str = ""
    http_uri: str = ""
    http_path: str = ""  # uri minus the query string
    http_query: str = ""
    http_request_version: str = ""  # "HTTP/1.1"
    http_user_agent: str = ""
    http_cookie: str = ""
    http_referer: str = ""
    http_request_content_type: str = ""
    http_request_content_length: str = ""
    http_request_headers: dict[str, str] = field(default_factory=dict)
    http_response_seen: bool = False
    http_response_code: str = ""
    http_response_phrase: str = ""
    http_response_content_type: str = ""
    http_response_content_length: str = ""
    http_response_headers: dict[str, str] = field(default_factory=dict)
    f5_reset_causes: list[str] = field(default_factory=list)
    # TLS handshake details (mostly tshark-sourced).
    tls_cert_subject: str = ""
    tls_cert_issuer: str = ""
    tls_supported_groups: str = ""  # curves / groups
    tls_signature_algos: str = ""
    # F5 trailer peer-side info (populated for `tcpdump -i <vlan>:np` captures
    # where each packet carries the proxied peer-side 5-tuple in the trailer).
    peer_remote_ip: str = ""
    peer_remote_port: int = 0
    peer_local_ip: str = ""
    peer_local_port: int = 0

    @property
    def key(self) -> tuple[str, int, str, int, int]:
        return (self.src_ip, self.src_port, self.dst_ip, self.dst_port, self.proto)

    @property
    def proto_name(self) -> str:
        return {6: "tcp", 17: "udp", 1: "icmp", 58: "icmpv6"}.get(self.proto, str(self.proto))

    def summary(self) -> str:
        parts = [
            f"{self.src_ip}:{self.src_port} -> {self.dst_ip}:{self.dst_port}",
            self.proto_name,
            f"{self.packets} pkt",
        ]
        if self.tcp_syn:
            parts.append("SYN")
        if self.tcp_synack:
            parts.append("SYN-ACK")
        if self.tcp_rst:
            parts.append(f"RST x{self.tcp_rst_count}")
        if self.tls_clienthello or self.tls_chosen_version:
            tls = "TLS"
            if self.tls_chosen_version:
                tls += f"/{self.tls_chosen_version}"
            elif self.tls_version:
                tls += f"/{self.tls_version}"
            if self.tls_sni:
                tls += f" SNI={self.tls_sni}"
            if self.tls_chosen_cipher:
                tls += f" cipher={self.tls_chosen_cipher}"
            if self.tls_alpn:
                tls += f" alpn={self.tls_alpn}"
            parts.append(tls)
        if self.http_request_seen:
            http = "HTTP"
            if self.http_method:
                http += f" {self.http_method}"
            if self.http_host:
                http += f" Host={self.http_host}"
            if self.http_uri:
                http += f" {self.http_uri}"
            parts.append(http)
        if self.http_response_code:
            parts.append(f"HTTP {self.http_response_code}")
        elif self.http_response_seen:
            parts.append("HTTP response")
        if self.f5_reset_causes:
            parts.append("f5-rst:" + ";".join(self.f5_reset_causes[:2]))
        return " | ".join(parts)


@dataclass(frozen=True, slots=True)
class Connection:
    """A bidirectional TCP/UDP conversation, formed by pairing two flows.

    The ``client`` side is the SYN-bearer (or, for non-TCP and SYN-less
    captures, the first-seen direction).  ``server`` is the reverse
    5-tuple if the response side appears in the capture, otherwise
    ``None`` (one-direction capture).  The connection's ``key`` is the
    canonical ordered pair so that re-pairing is idempotent.
    """

    client: Flow
    server: Flow | None = None

    @property
    def proto(self) -> int:
        return self.client.proto

    @property
    def proto_name(self) -> str:
        return self.client.proto_name

    @property
    def reset_side(self) -> str:
        if self.client.tcp_rst and self.server and self.server.tcp_rst:
            return "both"
        if self.client.tcp_rst:
            return "client"
        if self.server and self.server.tcp_rst:
            return "server"
        return ""

    def reset_causes(self) -> list[str]:
        out: list[str] = []
        out.extend(self.client.f5_reset_causes)
        if self.server is not None:
            out.extend(self.server.f5_reset_causes)
        # de-dup while preserving order
        seen: set[str] = set()
        result: list[str] = []
        for c in out:
            if c not in seen:
                seen.add(c)
                result.append(c)
        return result

    def summary(self) -> str:
        head = (
            f"{self.client.src_ip}:{self.client.src_port} <-> "
            f"{self.client.dst_ip}:{self.client.dst_port} "
            f"({self.proto_name})"
        )
        c_pkts = self.client.packets
        s_pkts = self.server.packets if self.server is not None else 0
        head += f" | client→ {c_pkts} pkt"
        if self.server is not None:
            head += f", server→ {s_pkts} pkt"
        if self.client.tls_sni:
            head += f" | SNI={self.client.tls_sni}"
        if self.client.tls_chosen_version or self.client.tls_version:
            head += f" | TLS={self.client.tls_chosen_version or self.client.tls_version}"
        if self.client.http_method:
            head += f" | HTTP {self.client.http_method} {self.client.http_uri}"
        if self.server is not None and self.server.http_response_code:
            head += f" -> {self.server.http_response_code}"
        if self.reset_side:
            head += f" | RST({self.reset_side})"
        return head


@dataclass(frozen=True, slots=True)
class Session:
    """A logical BIG-IP-mediated conversation: client↔VIP plus VIP↔server.

    On `tcpdump -i <vlan>:np` captures, every packet that crosses TMM
    is emitted twice — once on the front (client-facing) side and once
    on the back (pool-member-facing) side, each carrying the peer
    5-tuple in its F5 ethernet trailer.  :func:`pair_sessions` groups
    those into one Session: ``front`` is the client↔VIP Connection,
    ``back`` is the TMM↔pool-member Connection (or ``None`` if the
    capture point only saw one side).

    For captures without ``:np`` (single-side capture) the Session
    holds a ``front`` Connection and ``back=None``.
    """

    front: Connection
    back: Connection | None = None

    @property
    def proto(self) -> int:
        return self.front.proto

    @property
    def proto_name(self) -> str:
        return self.front.proto_name

    def reset_side(self) -> str:
        if self.back is not None and self.back.reset_side:
            return f"server-side ({self.back.reset_side})"
        return self.front.reset_side or ""

    def reset_causes(self) -> list[str]:
        causes = list(self.front.reset_causes())
        if self.back is not None:
            for c in self.back.reset_causes():
                if c not in causes:
                    causes.append(c)
        return causes

    def summary(self) -> str:
        head = f"front: {self.front.summary()}"
        if self.back is not None:
            head += f"\n         back:  {self.back.summary()}"
        return head


@dataclass(frozen=True, slots=True)
class SessionExplain:
    """Per-session explanation: which VS matched, event chain, RST analysis."""

    session: Session
    matched_vs: str = ""
    matched_partition: str = ""
    profile_chain: tuple[str, ...] = ()
    pool_selected: str = ""  # pool path observed via the back-side flow's dst
    snat_observed: str = ""  # SNAT IP observed if back.client.src_ip != front.client.src_ip
    event_sequence: tuple[str, ...] = ()
    event_blocks: tuple[tuple[str, str, str], ...] = ()  # (rule_path, event, body)
    # Per event-block: (rule_path, event, [(line, command, captured_value), ...]).
    event_annotations: tuple[tuple[str, str, tuple[tuple[str, str, str], ...]], ...] = ()
    ltm_policies: tuple[str, ...] = ()
    # Per-policy evaluation against the captured request state.  Empty
    # when no LTM policies are attached to the matched VS, when an
    # attached path is unresolved (e.g. a short name we couldn't
    # resolve to a full path), or when the policy body wasn't parsed
    # into ``BigipConfig.policies``.  When evaluation runs, every
    # parsed policy yields a ``PolicyDecision`` regardless of whether
    # the captured state was sufficient — individual conditions
    # surface their unevaluable-ness via ``ConditionTrace.note``.
    policy_decisions: tuple[PolicyDecision, ...] = ()
    apm_profile: str = ""
    gtm_wide_ips: tuple[str, ...] = ()
    explain_text: str = ""
    reset_analysis: str = ""  # human-readable narrative of why connection ended
    # Outcome of running the iRule under the C-tcl test harness, when
    # ``--simulate`` is passed.  Empty/blank if simulation was disabled
    # or failed to start.
    simulated_pool: str = ""
    simulated_node: str = ""
    simulated_response_committed: bool = False
    simulated_logs: tuple[str, ...] = ()
    simulated_decisions: tuple[tuple[str, str, str], ...] = ()
    simulation_error: str = ""


@dataclass(frozen=True, slots=True)
class ExplainFlowReport:
    pcap_path: str
    flow_count: int
    session_count: int
    matched_count: int
    sessions: tuple[SessionExplain, ...] = ()
    text_report: str = ""
    used_tshark: bool = False
    keylog_path: str = ""
    tshark_filter: str = ""
