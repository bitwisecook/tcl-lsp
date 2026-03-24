"""iApps dialect diagnostic codes (IAPP-series). All internal."""

from core.common.codes import diag

IAPP7001 = diag(
    "IAPP7001", "iApps template validation — missing section.", section="error", internal=True
)
IAPP7002 = diag(
    "IAPP7002", "iApps template validation — invalid reference.", section="error", internal=True
)
IAPP7003 = diag(
    "IAPP7003", "iApps template validation — deprecated syntax.", section="error", internal=True
)
