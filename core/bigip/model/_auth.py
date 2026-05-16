"""Typed projection for the auth module (``auth.*``).

Partitions, users, password / password-policy singletons,
auth sources, remote roles / users, login-failure tracking,
LDAP / RADIUS / TACACS+ / cert-LDAP / APM auth providers.
"""

from __future__ import annotations

from dataclasses import dataclass

from ...analysis.semantic_model import Range


@dataclass(frozen=True, slots=True)
class BigipAuthPartition:
    """An ``auth partition <name>`` object — administrative partition."""

    name: str
    full_path: str
    description: str = ""
    default_route_domain: str = ""
    inherited_traffic_group: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipAuthUser:
    """An ``auth user <name>`` object — local user account.

    ``partition_access`` surfaces the keys of the ``partition-access
    { ... }`` sub-block (each key is a partition name or
    ``all-partitions``); the per-key ``role`` value is not modelled
    in v1.
    """

    name: str
    full_path: str
    description: str = ""
    partition: str = ""
    shell: str = ""
    encrypted_password: str = ""
    partition_access: tuple[str, ...] = ()
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipAuthPassword:
    """The ``auth password`` singleton."""

    name: str = ""
    full_path: str = ""
    expiration_warning: str = ""
    minimum_length: str = ""
    policy: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipAuthPasswordPolicy:
    """The ``auth password-policy`` singleton."""

    name: str = ""
    full_path: str = ""
    expiration_warning: str = ""
    max_duration: str = ""
    max_login_failures: str = ""
    min_duration: str = ""
    minimum_length: str = ""
    minimum_regular_characters: str = ""
    password_memory: str = ""
    policy_enforcement: str = ""
    required_lowercase: str = ""
    required_numeric: str = ""
    required_special: str = ""
    required_uppercase: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipAuthSource:
    """The ``auth source`` singleton — selects the authentication backend."""

    name: str = ""
    full_path: str = ""
    fallback: str = ""
    type_: str = ""  # ``local`` / ``radius`` / ``ldap`` / ``tacacs`` / …
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipAuthRemoteRole:
    """The ``auth remote-role`` singleton.

    ``role_info`` surfaces the top-level keys of the ``role-info {
    ... }`` sub-block (one per remote-role entry); per-key
    attributes (``role``, ``console``, ``deny``, …) are not modelled
    in v1.
    """

    name: str = ""
    full_path: str = ""
    role_info: tuple[str, ...] = ()
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipAuthRemoteUser:
    """The ``auth remote-user`` singleton."""

    name: str = ""
    full_path: str = ""
    default_partition: str = ""
    default_role: str = ""
    remote_console_access: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipAuthLoginFailures:
    """The ``auth login-failures`` singleton."""

    name: str = ""
    full_path: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipAuthLdap:
    """An ``auth ldap <name>`` object — LDAP authentication profile."""

    name: str = ""
    full_path: str = ""
    bind_dn: str = ""
    bind_pw: str = ""
    bind_timeout: str = ""
    check_host_attr: str = ""
    check_roles_group: str = ""
    filter_: str = ""
    group_dn: str = ""
    group_member_attribute: str = ""
    idle_timeout: str = ""
    ignore_auth_info_unavail: str = ""
    ignore_unknown_user: str = ""
    login_attribute: str = ""
    port: str = ""
    scope: str = ""
    search_base_dn: str = ""
    search_timeout: str = ""
    servers: tuple[str, ...] = ()
    ssl: str = ""
    ssl_ca_cert: str = ""
    ssl_check_peer: str = ""
    ssl_client_cert: str = ""
    ssl_client_key: str = ""
    user_template: str = ""
    version: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipAuthRadius:
    """An ``auth radius <name>`` object — RADIUS authentication profile."""

    name: str = ""
    full_path: str = ""
    service_type: str = ""
    servers: tuple[str, ...] = ()  # PathRefs → auth radius-server
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipAuthRadiusServer:
    """An ``auth radius-server <name>`` object."""

    name: str = ""
    full_path: str = ""
    server: str = ""
    port: str = ""
    secret: str = ""
    timeout: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipAuthTacacs:
    """An ``auth tacacs <name>`` object — TACACS+ authentication profile."""

    name: str = ""
    full_path: str = ""
    protocol: str = ""
    secret: str = ""
    service: str = ""
    servers: tuple[str, ...] = ()  # bare hostnames / IPs, not full-paths
    accounting: str = ""
    authentication: str = ""
    debug: str = ""
    encryption: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipAuthCertLdap:
    """An ``auth cert-ldap <name>`` object — certificate-LDAP profile."""

    name: str = ""
    full_path: str = ""
    bind_dn: str = ""
    bind_pw: str = ""
    bind_timeout: str = ""
    idle_timeout: str = ""
    login_attribute: str = ""
    port: str = ""
    scope: str = ""
    search_base_dn: str = ""
    search_timeout: str = ""
    servers: tuple[str, ...] = ()
    ssl: str = ""
    user_template: str = ""
    version: str = ""
    range: Range | None = None


@dataclass(frozen=True, slots=True)
class BigipAuthApmAuth:
    """An ``auth apm-auth <name>`` object — APM auth profile binding."""

    name: str = ""
    full_path: str = ""
    profile: str = ""  # PathRef → apm policy access-policy
    range: Range | None = None


# Aggregate config inventory
