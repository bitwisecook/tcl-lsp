"""Typed projection for the PEM module (``pem.*``).

Policies / rules, listeners, forwarding / interception / service-
chain endpoints, profiles, rating groups.  Long-tail ``pem.*``
kinds share :class:`BigipMinimalObject` via
:data:`BigipPemMinimalObject`.
"""

from __future__ import annotations

from dataclasses import dataclass

from shared.diagnostic import Range


@dataclass(frozen=True, slots=True)
class BigipPemPolicy:
    """A ``pem policy`` object — a subscriber-traffic policy.

    Rule bodies are kept as raw stanza names in ``rules`` so callers
    can interrogate which rules exist without modelling the full
    condition / action grammar.
    """

    name: str
    full_path: str
    description: str = ""
    rules: tuple[str, ...] = ()
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipPemRule:
    """A ``pem irule`` object — a PEM-context iRule."""

    name: str
    full_path: str
    source: str = ""
    description: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipPemListener:
    """A ``pem listener`` object — applies PEM to a set of virtual servers."""

    name: str
    full_path: str
    description: str = ""
    profile_spm: str = ""  # PathRef → pem profile spm
    profile_subscriber_mgmt: str = ""  # PathRef → pem profile subscriber-mgmt
    virtual_servers: tuple[str, ...] = ()  # PathRefs → ltm virtual
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipPemForwardingEndpoint:
    """A ``pem forwarding-endpoint`` object — a downstream forwarding target."""

    name: str
    full_path: str
    description: str = ""
    pool: str = ""  # PathRef → ltm pool
    snat_pool: str = ""  # PathRef → ltm snatpool
    source_ip: str = ""
    destination_ip: str = ""
    type_: str = ""
    persistence: str = ""
    translate_address: str = ""
    translate_service: str = ""
    fallback: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipPemInterceptionEndpoint:
    """A ``pem interception-endpoint`` object — an upstream tap destination."""

    name: str
    full_path: str
    description: str = ""
    pool: str = ""  # PathRef → ltm pool
    persistence: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipPemServiceChainEndpoint:
    """A ``pem service-chain-endpoint`` object — an ordered chain of endpoints."""

    name: str
    full_path: str
    description: str = ""
    service_endpoints: tuple[str, ...] = ()
    steering_policy: str = ""  # PathRef → pem policy
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipPemProfile:
    """A ``pem profile <type>`` object — bundles every PEM profile sub-type.

    The ``profile_type`` field carries the sub-type token
    (``diameter-endpoint``, ``radius-aaa``, ``spm``, ``subscriber-mgmt``)
    so callers can filter without reaching back into the kind string.
    """

    name: str
    full_path: str
    profile_type: str = ""
    defaults_from: str = ""  # PathRef → pem profile <same type>
    description: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipPemRatingGroup:
    """A ``pem quota-mgmt rating-group`` object."""

    name: str
    full_path: str
    description: str = ""
    rating_group_id: str = ""
    default_quota: str = ""
    default_quota_holding_time: str = ""
    default_validity_time: str = ""
    default_threshold: str = ""
    total_octets: str = ""
    input_octets: str = ""
    output_octets: str = ""
    time: str = ""
    consumption_time: str = ""
    usage_time: str = ""
    volume: str = ""
    range: Range | None = None


# auth.* — typed projection for the authentication module.
