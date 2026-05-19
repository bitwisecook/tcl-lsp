"""Typed projection for the system module (``sys.*``).

DNS / NTP / SNMP / global-settings singletons, provisioning,
folders, SSL cert / key files, management routes.  Long-tail
``sys.*`` kinds share :class:`BigipMinimalObject` via
:data:`BigipSysMinimalObject`.
"""

from __future__ import annotations

from dataclasses import dataclass

from shared.diagnostic import Range

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
class BigipSysNtpRestrict:
    """One ``sys ntp.restrict`` entry — per-network NTP ACL rule.

    Each entry names an address + mask plus a set of behaviour flags
    (``kod``, ``limited``, ``no-modify``, ``no-query``, ``no-trust``,
    …).  The flag set is surfaced as a string tuple of just the names
    that are ``enabled`` for this entry; absent / ``disabled`` flags
    are filtered out so consumers can do ``"no-query" in r.flags``.
    """

    name: str = ""  # keyed-block name (the entry's identifier)
    address: str = ""
    mask: str = ""
    default_entry: str = ""  # ``enabled`` / ``disabled``
    flags: tuple[str, ...] = ()
    description: str = ""

    def query_fields(self) -> dict[str, object]:
        """DSL-facing field map (TMSH-spelt keys)."""
        return {
            "name": self.name,
            "address": self.address,
            "mask": self.mask,
            "default-entry": self.default_entry,
            "flags": list(self.flags),
            "description": self.description,
        }


@dataclass(frozen=True, slots=True)
class BigipSysNtp:
    """The ``sys ntp`` singleton — NTP server settings."""

    name: str = ""
    full_path: str = ""
    servers: tuple[str, ...] = ()
    timezone: str = ""
    restrict: tuple[BigipSysNtpRestrict, ...] = ()
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSysSnmpUser:
    """One ``sys snmp.users`` entry — an SNMPv3 user definition."""

    name: str = ""
    username: str = ""
    security_level: str = ""  # ``no-auth-no-privacy`` / ``auth-no-privacy`` / ``auth-privacy``
    auth_protocol: str = ""  # ``md5`` / ``sha`` / ``none``
    privacy_protocol: str = ""  # ``aes`` / ``des`` / ``none``
    oid_subset: str = ""
    description: str = ""

    def query_fields(self) -> dict[str, object]:
        return {
            "name": self.name,
            "username": self.username,
            "security-level": self.security_level,
            "auth-protocol": self.auth_protocol,
            "privacy-protocol": self.privacy_protocol,
            "oid-subset": self.oid_subset,
            "description": self.description,
        }


@dataclass(frozen=True, slots=True)
class BigipSysSnmpTrap:
    """One ``sys snmp.traps`` entry — SNMP notification destination."""

    name: str = ""
    host: str = ""
    port: str = ""
    version: str = ""  # ``1`` / ``2c`` / ``3``
    community: str = ""
    security_name: str = ""
    security_level: str = ""
    auth_protocol: str = ""
    privacy_protocol: str = ""
    network: str = ""
    description: str = ""

    def query_fields(self) -> dict[str, object]:
        return {
            "name": self.name,
            "host": self.host,
            "port": self.port,
            "version": self.version,
            "community": self.community,
            "security-name": self.security_name,
            "security-level": self.security_level,
            "auth-protocol": self.auth_protocol,
            "privacy-protocol": self.privacy_protocol,
            "network": self.network,
            "description": self.description,
        }


@dataclass(frozen=True, slots=True)
class BigipSysSnmpProcessMonitor:
    """One ``sys snmp.process-monitors`` entry."""

    name: str = ""
    process: str = ""
    max_processes: str = ""
    min_processes: str = ""
    description: str = ""

    def query_fields(self) -> dict[str, object]:
        return {
            "name": self.name,
            "process": self.process,
            "max-processes": self.max_processes,
            "min-processes": self.min_processes,
            "description": self.description,
        }


@dataclass(frozen=True, slots=True)
class BigipSysSnmpDiskMonitor:
    """One ``sys snmp.disk-monitors`` entry."""

    name: str = ""
    partition: str = ""
    min_space: str = ""
    description: str = ""

    def query_fields(self) -> dict[str, object]:
        return {
            "name": self.name,
            "partition": self.partition,
            "min-space": self.min_space,
            "description": self.description,
        }


@dataclass(frozen=True, slots=True)
class BigipSysSnmp:
    """The ``sys snmp`` singleton — SNMP agent settings."""

    name: str = ""
    full_path: str = ""
    agent_addresses: tuple[str, ...] = ()
    communities: tuple[str, ...] = ()  # full-paths of community sub-objects
    sys_contact: str = ""
    sys_location: str = ""
    sys_services: str = ""
    trap_community: str = ""
    users: tuple[BigipSysSnmpUser, ...] = ()
    traps: tuple[BigipSysSnmpTrap, ...] = ()
    process_monitors: tuple[BigipSysSnmpProcessMonitor, ...] = ()
    disk_monitors: tuple[BigipSysSnmpDiskMonitor, ...] = ()
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
