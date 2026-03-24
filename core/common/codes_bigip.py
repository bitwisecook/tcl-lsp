"""BIG-IP dialect diagnostic codes (BIGIP-series). All internal."""

from core.common.codes import diag

BIGIP6001 = diag(
    "BIGIP6001", "BIG-IP config validation — missing field.", section="error", internal=True
)
BIGIP6002 = diag(
    "BIGIP6002", "BIG-IP config validation — invalid value.", section="error", internal=True
)
BIGIP6003 = diag(
    "BIGIP6003", "BIG-IP config validation — deprecated syntax.", section="error", internal=True
)
BIGIP6004 = diag(
    "BIGIP6004", "BIG-IP config validation — unknown property.", section="error", internal=True
)
BIGIP6005 = diag(
    "BIGIP6005", "BIG-IP config validation — type mismatch.", section="error", internal=True
)
BIGIP6006 = diag(
    "BIGIP6006", "BIG-IP config validation — missing reference.", section="error", internal=True
)
BIGIP6007 = diag(
    "BIGIP6007", "BIG-IP config validation — duplicate entry.", section="error", internal=True
)
BIGIP6008 = diag(
    "BIGIP6008", "BIG-IP config validation — invalid range.", section="error", internal=True
)
BIGIP6009 = diag(
    "BIGIP6009", "BIG-IP config validation — unsupported feature.", section="error", internal=True
)
BIGIP6010 = diag(
    "BIGIP6010", "BIG-IP config validation — security concern.", section="error", internal=True
)
BIGIP6011 = diag(
    "BIGIP6011", "BIG-IP config validation — compatibility warning.", section="error", internal=True
)
