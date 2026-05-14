from __future__ import annotations

from ..models import (
    BigipObjectKindSpec,
    BigipObjectSpec,
    BigipPropertySpec,
)
from ._base import register


@register
def register_spec() -> BigipObjectSpec:
    return BigipObjectSpec(
        kind_spec=BigipObjectKindSpec(
            "sys_syslog",
            module="sys",
            object_types=("syslog",),
        ),
        header_types=(("sys", "syslog"),),
        properties=(
            BigipPropertySpec(
                name="auth-priv-from",
                value_type="enum",
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warning"),
            ),
            BigipPropertySpec(
                name="auth-priv-to",
                value_type="enum",
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warning"),
            ),
            BigipPropertySpec(
                name="clustered-host-name",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="clustered-message-slot",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="console-log",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="cron-from",
                value_type="enum",
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warning"),
            ),
            BigipPropertySpec(
                name="cron-to",
                value_type="enum",
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warning"),
            ),
            BigipPropertySpec(
                name="daemon-from",
                value_type="enum",
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warning"),
            ),
            BigipPropertySpec(
                name="daemon-to",
                value_type="enum",
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warning"),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="include", value_type="string"),
            BigipPropertySpec(
                name="iso-date",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="kern-from",
                value_type="enum",
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warning"),
            ),
            BigipPropertySpec(
                name="kern-to",
                value_type="enum",
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warning"),
            ),
            BigipPropertySpec(
                name="local6-from",
                value_type="enum",
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warning"),
            ),
            BigipPropertySpec(
                name="local6-to",
                value_type="enum",
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warning"),
            ),
            BigipPropertySpec(
                name="mail-from",
                value_type="enum",
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warning"),
            ),
            BigipPropertySpec(
                name="mail-to",
                value_type="enum",
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warning"),
            ),
            BigipPropertySpec(
                name="messages-from",
                value_type="enum",
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warning"),
            ),
            BigipPropertySpec(
                name="messages-to",
                value_type="enum",
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warning"),
            ),
            BigipPropertySpec(
                name="remote-servers",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(name="host", value_type="unknown", in_sections=("remote-servers",)),
            BigipPropertySpec(
                name="local-ip",
                value_type="string",
                in_sections=("remote-servers",),
            ),
            BigipPropertySpec(
                name="remote-port",
                value_type="unknown",
                in_sections=("remote-servers",),
            ),
            BigipPropertySpec(
                name="user-log-from",
                value_type="enum",
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warning"),
            ),
            BigipPropertySpec(
                name="user-log-to",
                value_type="enum",
                enum_values=("alert", "crit", "debug", "emerg", "err", "info", "notice", "warning"),
            ),
        ),
    )
