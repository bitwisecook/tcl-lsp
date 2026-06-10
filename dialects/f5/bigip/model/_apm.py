"""Typed projection for the APM module (``apm.*``).

Ephemeral auth, OAuth, policy items / agents / customisation,
report definitions.  Long-tail ``apm.*`` kinds share
:class:`BigipMinimalObject` via :data:`BigipApmMinimalObject`.
"""

from __future__ import annotations

from dataclasses import dataclass

from shared.diagnostic import Range


@dataclass(frozen=True, slots=True)
class BigipApmEphemeralAuthSshSecurityConfig:
    """A ``apm ephemeral-auth ssh-security-config`` object.

    The body lists ciphers / hmacs / kex-methods / compressions as
    numerically-keyed sub-blocks; we surface flattened name lists in
    v1 and leave the per-entry detail to the source view.
    """

    name: str
    full_path: str
    ciphers: tuple[str, ...] = ()
    hmacs: tuple[str, ...] = ()
    kex_methods: tuple[str, ...] = ()
    compressions: tuple[str, ...] = ()
    description: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipApmOauthDbInstance:
    """A ``apm oauth db-instance`` object."""

    name: str
    full_path: str
    description: str = ""
    db_name: str = ""
    purge_frequency: str = ""
    purge_time: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipApmPolicyAccessPolicy:
    """A ``apm policy access-policy`` object — a per-flow access policy."""

    name: str
    full_path: str
    start_item: str = ""  # PathRef → apm policy policy-item
    default_ending: str = ""  # PathRef → apm policy policy-item
    items: tuple[str, ...] = ()  # PathRefs → apm policy policy-item
    description: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipApmPolicyCustomizationSource:
    """A ``apm policy customization-source`` object."""

    name: str
    full_path: str
    description: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipApmPolicyItem:
    """A ``apm policy policy-item`` object — a node in an access policy.

    ``agents`` holds the full-paths of the agent sub-block keys; the
    nested ``rules { { … } { … } }`` anonymous sequence is not
    modelled in v1 (use the source view).
    """

    name: str
    full_path: str
    caption: str = ""
    color: str = ""
    item_type: str = ""  # action | ending
    agents: tuple[str, ...] = ()
    description: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipApmPolicyAgent:
    """An ``apm policy agent <type>`` object.

    The agent type (``ending-allow``, ``ending-deny``, ``kerberos``,
    …) is captured in ``agent_type`` so callers can filter without
    reaching into the kind string.
    """

    name: str
    full_path: str
    agent_type: str = ""
    customization_group: str = ""
    auth: str = ""
    max_logon_attempt: str = ""
    auth_max_logon_attempt: str = ""
    fetch_nested_groups: str = ""
    fetch_primary_groups: str = ""
    password_source: str = ""
    query: str = ""
    query_attrname: str = ""
    query_filter: str = ""
    server: str = ""
    show_extended_error: str = ""
    upn: str = ""
    username_source: str = ""
    attribute_consuming_service: str = ""
    attr_consuming_service_session_var: str = ""
    hints: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipApmReportDefaultReport:
    """The ``apm report default-report`` singleton."""

    name: str = ""
    full_path: str = ""
    report_name: str = ""
    user: str = ""
    range: Range | None = None


# cm.* — typed projection for the Cluster Manager module.
