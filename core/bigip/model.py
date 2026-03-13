"""Data model for F5 BIG-IP configuration objects.

Represents the parsed inventory of a ``bigip.conf`` (or SCF) file:
virtual servers, pools, data-groups, profiles, iRules, nodes, monitors,
SNAT pools, and persistence profiles.
"""

from __future__ import annotations

from collections.abc import Mapping
from dataclasses import dataclass, field
from enum import Enum, auto

from ..analysis.semantic_model import Range

# Enums


class DataGroupType(Enum):
    """Whether a data-group is stored inline or in an external file."""

    INTERNAL = auto()
    EXTERNAL = auto()


class ProfileType(Enum):
    """Broad classification of BIG-IP profile types."""

    HTTP = auto()
    TCP = auto()
    UDP = auto()
    CLIENT_SSL = auto()
    SERVER_SSL = auto()
    FTP = auto()
    DNS = auto()
    SIP = auto()
    DIAMETER = auto()
    FIX = auto()
    RADIUS = auto()
    MQTT = auto()
    WEBSOCKET = auto()
    STREAM = auto()
    HTML = auto()
    REWRITE = auto()
    FASTHTTP = auto()
    FASTL4 = auto()
    ONE_CONNECT = auto()
    PERSISTENCE = auto()
    OTHER = auto()


# Parsed objects


@dataclass(frozen=True, slots=True)
class BigipDataGroup:
    """A ``ltm data-group internal|external`` object."""

    name: str
    full_path: str  # e.g. "/Common/my_dg"
    kind: DataGroupType = DataGroupType.INTERNAL
    value_type: str = ""  # "string", "ip", "integer"
    records: tuple[str, ...] = ()  # record names/keys
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipPoolMember:
    """A single member entry inside a ``ltm pool``."""

    name: str  # e.g. "/Common/10.0.0.1:80"
    address: str = ""
    port: int = 0
    monitor: str = ""


@dataclass(frozen=True, slots=True)
class BigipPool:
    """A ``ltm pool`` object."""

    name: str
    full_path: str
    module: str = "ltm"
    members: tuple[BigipPoolMember, ...] = ()
    monitor: str = ""
    load_balancing_mode: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipNode:
    """A ``ltm node`` object."""

    name: str
    full_path: str
    address: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipProfile:
    """A ``ltm profile <type>`` object."""

    name: str
    full_path: str
    profile_type: ProfileType = ProfileType.OTHER
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipMonitor:
    """A ``ltm monitor <type>`` object."""

    name: str
    full_path: str
    monitor_type: str = ""  # "http", "tcp", "https", etc.
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipSnatPool:
    """A ``ltm snatpool`` object."""

    name: str
    full_path: str
    members: tuple[str, ...] = ()
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipPersistence:
    """A ``ltm persistence <type>`` object."""

    name: str
    full_path: str
    persistence_type: str = ""  # "cookie", "source-addr", "ssl", etc.
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipRule:
    """A ``ltm rule`` object — an iRule embedded in the config."""

    name: str
    full_path: str
    source: str = ""  # the raw Tcl body
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipVirtualServer:
    """A ``ltm virtual`` object."""

    name: str
    full_path: str
    destination: str = ""
    pool: str = ""  # default pool path
    rules: tuple[str, ...] = ()  # attached iRule paths
    profiles: tuple[str, ...] = ()  # attached profile paths
    persist: tuple[str, ...] = ()  # persistence profile paths
    snatpool: str = ""
    source_address_translation: str = ""
    pool_range: Range | None = None
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipGenericObject:
    """A generic BIG-IP stanza retained when no specialised model exists."""

    module: str  # e.g. "net", "auth", "sys"
    object_type: str  # e.g. "route-domain", "partition", "user"
    identifier: str  # e.g. "/Common/0", "admin", or "" for singleton stanzas
    header: str
    range: Range | None = None


# Aggregate config inventory


@dataclass
class BigipConfig:
    """Complete parsed inventory of a BIG-IP configuration file."""

    data_groups: dict[str, BigipDataGroup] = field(default_factory=dict)
    pools: dict[str, BigipPool] = field(default_factory=dict)
    virtual_servers: dict[str, BigipVirtualServer] = field(default_factory=dict)
    nodes: dict[str, BigipNode] = field(default_factory=dict)
    profiles: dict[str, BigipProfile] = field(default_factory=dict)
    monitors: dict[str, BigipMonitor] = field(default_factory=dict)
    snat_pools: dict[str, BigipSnatPool] = field(default_factory=dict)
    persistence: dict[str, BigipPersistence] = field(default_factory=dict)
    rules: dict[str, BigipRule] = field(default_factory=dict)
    generic_objects: dict[str, BigipGenericObject] = field(default_factory=dict)

    def resolve_name(self, name: str, objects: Mapping[str, object]) -> str | None:
        """Resolve a possibly-short name to a full path in *objects*.

        BIG-IP configs use full paths like ``/Common/my_pool`` but iRules
        may reference just ``my_pool``.  This tries exact match first, then
        falls back to a suffix match.
        """
        if name in objects:
            return name
        # Try with /Common/ prefix
        candidate = f"/Common/{name}"
        if candidate in objects:
            return candidate
        # Suffix match: look for any key ending with /<name>
        suffix = f"/{name}"
        for key in objects:
            if key.endswith(suffix):
                return key
        return None

    def resolve_pool(self, name: str) -> str | None:
        return self.resolve_name(name, self.pools)

    def resolve_data_group(self, name: str) -> str | None:
        return self.resolve_name(name, self.data_groups)

    def resolve_snat_pool(self, name: str) -> str | None:
        return self.resolve_name(name, self.snat_pools)

    def resolve_persistence(self, name: str) -> str | None:
        return self.resolve_name(name, self.persistence)

    def resolve_rule(self, name: str) -> str | None:
        return self.resolve_name(name, self.rules)

    def resolve_profile(self, name: str) -> str | None:
        return self.resolve_name(name, self.profiles)

    def resolve_generic_object(
        self,
        name: str,
        *,
        module: str | None = None,
        object_types: tuple[str, ...] | None = None,
    ) -> str | None:
        """Resolve a generic BIG-IP object key by identifier/name."""
        clean = name.strip()
        if not clean:
            return None

        def _matches(obj: BigipGenericObject) -> bool:
            if module is not None and obj.module != module:
                return False
            if object_types is not None and obj.object_type not in object_types:
                return False
            ident = obj.identifier
            if ident == clean:
                return True
            if clean.startswith("/") and ident.endswith(clean):
                return True
            if not clean.startswith("/"):
                if ident.endswith(f"/{clean}") or ident == clean:
                    return True
            return False

        for key, obj in self.generic_objects.items():
            if _matches(obj):
                return key
        return None

    def profiles_for_virtual(self, vs_name: str) -> list[BigipProfile]:
        """Return resolved profile objects attached to a virtual server."""
        vs = self.virtual_servers.get(vs_name)
        if vs is None:
            return []
        result: list[BigipProfile] = []
        for pref in vs.profiles:
            resolved = self.resolve_profile(pref)
            if resolved and resolved in self.profiles:
                result.append(self.profiles[resolved])
        return result

    def profile_types_for_virtual(self, vs_name: str) -> frozenset[ProfileType]:
        """Return the set of profile types attached to a virtual server."""
        return frozenset(p.profile_type for p in self.profiles_for_virtual(vs_name))
