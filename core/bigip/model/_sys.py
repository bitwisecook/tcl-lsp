"""Typed projection for the system module (``sys.*``).

DNS / NTP / SNMP / global-settings singletons, provisioning,
folders, SSL cert / key files, management routes.  Long-tail
``sys.*`` kinds share :class:`BigipMinimalObject` via
:data:`BigipSysMinimalObject`.
"""

from __future__ import annotations

from dataclasses import dataclass

from ...analysis.semantic_model import Range

# sys.* — typed projection for the system module.
#
# Singletons (``sys dns``, ``sys ntp``, ``sys snmp``, ``sys
# global-settings``) have no full-path; they're stored with the empty
# string as the dict key so ``.sys.<kind>[]`` streams the one entry.


@dataclass(frozen=True, slots=True)
class BigipSysDns:
    """The ``sys dns`` singleton — DNS resolver settings."""

    name: str = ""
    full_path: str = ""
    name_servers: tuple[str, ...] = ()
    search: tuple[str, ...] = ()
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSysNtp:
    """The ``sys ntp`` singleton — NTP server settings."""

    name: str = ""
    full_path: str = ""
    servers: tuple[str, ...] = ()
    timezone: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSysSnmp:
    """The ``sys snmp`` singleton — SNMP agent settings."""

    name: str = ""
    full_path: str = ""
    agent_addresses: tuple[str, ...] = ()
    communities: tuple[str, ...] = ()  # full-paths of community sub-objects
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSysGlobalSettings:
    """The ``sys global-settings`` singleton."""

    name: str = ""
    full_path: str = ""
    hostname: str = ""
    gui_setup: str = ""
    mgmt_dhcp: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSysProvision:
    """A ``sys provision <module>`` object — module provisioning level.

    ``name`` is the bare module token (``ltm``, ``sslo``, ``urldb``);
    ``full_path`` mirrors it.
    """

    name: str
    full_path: str
    level: str = ""
    cpu_ratio: str = ""
    memory_ratio: str = ""
    disk_ratio: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSysFolder:
    """A ``sys folder`` object — partition / folder metadata."""

    name: str
    full_path: str
    device_group: str = ""
    traffic_group: str = ""
    hidden: str = ""
    description: str = ""
    inherited_device_group: str = ""
    inherited_traffic_group: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSysFileSslCert:
    """A ``sys file ssl-cert`` object."""

    name: str
    full_path: str
    source_path: str = ""
    cache_path: str = ""
    revision: str = ""
    description: str = ""
    issuer: str = ""
    subject: str = ""
    expiration_string: str = ""
    expiration_date: str = ""
    fingerprint: str = ""
    key_size: str = ""
    key_type: str = ""
    is_bundle: str = ""
    certificate_key_size: str = ""
    issuer_cert: str = ""  # PathRef → sys file ssl-cert
    serial_number: str = ""
    version: str = ""
    subject_alternative_name: str = ""
    bundle_certificates: tuple[str, ...] = ()
    cert_validation_options: tuple[str, ...] = ()
    cert_validators: tuple[str, ...] = ()
    checksum: str = ""
    mode: str = ""
    size: str = ""
    create_time: str = ""
    created_by: str = ""
    last_update_time: str = ""
    updated_by: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSysFileSslKey:
    """A ``sys file ssl-key`` object."""

    name: str
    full_path: str
    source_path: str = ""
    cache_path: str = ""
    revision: str = ""
    passphrase: str = ""
    description: str = ""
    key_size: str = ""
    key_type: str = ""
    security_type: str = ""
    checksum: str = ""
    mode: str = ""
    size: str = ""
    create_time: str = ""
    created_by: str = ""
    last_update_time: str = ""
    updated_by: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSysManagementRoute:
    """A ``sys management-route`` object."""

    name: str
    full_path: str
    gateway: str = ""
    network: str = ""
    mtu: str = ""
    description: str = ""
    range: Range | None = None


# security.* — typed projection for AFM / DDoS / device-id / inspection.
