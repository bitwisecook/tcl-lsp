"""Typed projection for the cluster-management module (``cm.*``).

Device certs / keys, devices, device-groups, traffic-groups,
trust-domains.
"""

from __future__ import annotations

from dataclasses import dataclass

from ...analysis.semantic_model import Range


@dataclass(frozen=True, slots=True)
class BigipCmCert:
    """A ``cm cert`` object — a device-trust certificate."""

    name: str
    full_path: str
    cache_path: str = ""
    checksum: str = ""
    revision: str = ""
    issuer: str = ""
    subject: str = ""
    subject_alternative_name: str = ""
    expiration_date: str = ""
    expiration_string: str = ""
    fingerprint: str = ""
    serial_number: str = ""
    version: str = ""
    key_type: str = ""
    certificate_key_size: str = ""
    is_bundle: str = ""
    email: str = ""
    source_path: str = ""
    system_path: str = ""
    size: str = ""
    mode: str = ""
    create_time: str = ""
    created_by: str = ""
    last_update_time: str = ""
    updated_by: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipCmKey:
    """A ``cm key`` object — the private key paired with a ``cm cert``."""

    name: str
    full_path: str
    cache_path: str = ""
    checksum: str = ""
    revision: str = ""
    key_size: str = ""
    key_type: str = ""
    security_type: str = ""
    source_path: str = ""
    system_path: str = ""
    size: str = ""
    mode: str = ""
    create_time: str = ""
    created_by: str = ""
    last_update_time: str = ""
    updated_by: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipCmDevice:
    """A ``cm device`` object.

    Only the core scalar identity / placement fields are surfaced; the
    bulky ``active-modules`` / ``optional-modules`` / ``time-limited-modules``
    lists (quoted multi-line bundles) are left to the source view.
    """

    name: str
    full_path: str
    hostname: str = ""
    management_ip: str = ""
    base_mac: str = ""
    build: str = ""
    edition: str = ""
    version: str = ""
    product: str = ""
    platform_id: str = ""
    chassis_id: str = ""
    marketing_name: str = ""
    self_device: str = ""  # "true" / "false" (text, not coerced)
    time_zone: str = ""
    cert: str = ""  # PathRef → cm cert
    key: str = ""  # PathRef → cm key
    description: str = ""
    comment: str = ""
    contact: str = ""
    location: str = ""
    mirror_ip: str = ""
    mirror_secondary_ip: str = ""
    multicast_interface: str = ""
    multicast_ip: str = ""
    multicast_port: str = ""
    unicast_address: tuple[str, ...] = ()
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipCmDeviceGroup:
    """A ``cm device-group`` object."""

    name: str
    full_path: str
    auto_sync: str = ""
    network_failover: str = ""
    hidden: str = ""
    devices: tuple[str, ...] = ()  # PathRefs → cm device
    description: str = ""
    type_: str = ""  # ``sync-failover`` / ``sync-only``
    save_on_auto_sync: str = ""
    full_load_on_sync: str = ""
    asm_sync: str = ""
    incremental_config_sync_size_max: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipCmTrafficGroup:
    """A ``cm traffic-group`` object."""

    name: str
    full_path: str
    unit_id: str = ""
    description: str = ""
    default_device: str = ""  # PathRef → cm device
    ha_load_factor: str = ""
    ha_order: tuple[str, ...] = ()  # PathRefs → cm device
    ha_group: str = ""  # PathRef → cm ha-group (TMSH-deprecated, still appears)
    auto_failback_enabled: str = ""
    auto_failback_time: str = ""
    mac: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipCmTrustDomain:
    """A ``cm trust-domain`` object."""

    name: str
    full_path: str
    ca_cert: str = ""  # PathRef → cm cert
    ca_cert_bundle: str = ""  # PathRef → cm cert
    ca_key: str = ""  # PathRef → cm key
    ca_devices: tuple[str, ...] = ()  # PathRefs → cm device
    guid: str = ""
    status: str = ""
    trust_group: str = ""  # PathRef → cm device-group
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipCmHaGroup:
    """A ``cm ha-group`` object — HA scoring rule for a traffic group.

    Sums weighted contributions from monitored pools / trunks; the
    traffic-group with the highest score (plus optional
    ``active-bonus``) is preferred for activation.  Fields modelled at
    the top level; per-pool / per-trunk weights are surfaced as
    ``pools`` / ``trunks`` PathRef tuples — the per-entry weight /
    threshold values live inside the source stanza and can be
    inspected via ``f5 grep`` when audit detail is needed.
    """

    name: str
    full_path: str
    description: str = ""
    enabled_state: str = ""  # ``enabled`` / ``disabled``
    active_bonus: str = ""
    pools: tuple[str, ...] = ()  # PathRefs → ltm pool (keyed-block names)
    trunks: tuple[str, ...] = ()  # PathRefs → net trunk (keyed-block names)
    range: Range | None = None


# gtm.* — typed projection for the Global Traffic Manager module.
