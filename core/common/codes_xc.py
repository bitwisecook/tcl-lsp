"""F5 XC translatability diagnostic codes (XC-series). All internal."""

from core.common.codes import diag

XC100 = diag("XC100", "Translatable — origin pool.", section="irules", internal=True)
XC101 = diag("XC101", "Translatable — HTTP redirect.", section="irules", internal=True)
XC102 = diag("XC102", "Translatable — HTTP response.", section="irules", internal=True)
XC103 = diag("XC103", "Translatable — header manipulation.", section="irules", internal=True)
XC105 = diag("XC105", "Translatable — cookie manipulation.", section="irules", internal=True)
XC106 = diag("XC106", "Translatable — URI rewrite.", section="irules", internal=True)
XC107 = diag("XC107", "Translatable — logging.", section="irules", internal=True)
XC200 = diag("XC200", "Partial — requires manual review.", section="irules", internal=True)
XC201 = diag("XC201", "Partial — complex conditional logic.", section="irules", internal=True)
XC203 = diag("XC203", "Partial — data group lookup.", section="irules", internal=True)
XC250 = diag("XC250", "Partial — persistence.", section="irules", internal=True)
XC300 = diag("XC300", "Untranslatable — no XC equivalent.", section="irules", internal=True)
XC301 = diag("XC301", "Untranslatable — complex feature.", section="irules", internal=True)
