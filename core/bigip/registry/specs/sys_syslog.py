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
            BigipPropertySpec(name="auth-priv-from", value_type="string"),
            BigipPropertySpec(name="notice", value_type="string"),
            BigipPropertySpec(name="auth-priv-to", value_type="string"),
            BigipPropertySpec(
                name="clustered-host-name", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="clustered-message-slot",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(name="cron-from", value_type="string"),
            BigipPropertySpec(name="cron-to", value_type="boolean"),
            BigipPropertySpec(name="daemon-from", value_type="string"),
            BigipPropertySpec(name="daemon-to", value_type="boolean"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="include", value_type="string"),
            BigipPropertySpec(
                name="iso-date", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="console-log", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(name="kern-from", value_type="boolean"),
            BigipPropertySpec(name="kern-to", value_type="boolean"),
            BigipPropertySpec(name="local6-from", value_type="boolean"),
            BigipPropertySpec(name="local6-to", value_type="boolean"),
            BigipPropertySpec(name="mail-from", value_type="boolean"),
            BigipPropertySpec(name="mail-to", value_type="boolean"),
            BigipPropertySpec(name="messages-from", value_type="string"),
            BigipPropertySpec(name="messages-to", value_type="boolean"),
            BigipPropertySpec(
                name="remote-servers",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="host",
                value_type="string",
                in_sections=("remote-servers",),
                pattern="^(?=.{1,253}$)(?:[A-Za-z0-9](?:[A-Za-z0-9-]{0,61}[A-Za-z0-9])?\\\\.)+[A-Za-z]{2,63}$",
            ),
            BigipPropertySpec(
                name="local-ip", value_type="string", in_sections=("remote-servers",)
            ),
            BigipPropertySpec(
                name="remote-port",
                value_type="integer",
                in_sections=("remote-servers",),
                min_value=0,
                max_value=65535,
            ),
            BigipPropertySpec(name="user-log-from", value_type="boolean"),
            BigipPropertySpec(name="user-log-to", value_type="boolean"),
        ),
    )
