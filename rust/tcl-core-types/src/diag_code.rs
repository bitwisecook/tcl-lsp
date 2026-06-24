//! Stable diagnostic-code identities (`W210`, `E001`, `O100`, `IRULE3102`, …).
//!
//! Every diagnostic the analyser, compiler-checks pipeline, CLI, and LSP emit is
//! tagged with one of these codes. Historically each site carried a bare string
//! literal; [`DiagCode`] makes the set a single typed vocabulary so a code can
//! no longer be mistyped, an unknown code is a parse error at the one boundary
//! that ingests user config, and the full catalogue is enumerable
//! ([`DiagCode::ALL`]) — which lets a test prove the published code table and
//! the emitted set never drift apart.
//!
//! The string spelling is the wire/UI contract (LSP `Diagnostic.code`, CLI
//! output, `--disable W123`, the docs); [`DiagCode::as_str`] / [`core::fmt::Display`]
//! produce it and [`core::str::FromStr`] parses it back.

use core::fmt;
use core::str::FromStr;

/// The coarse family a [`DiagCode`] belongs to, derived from its letter prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DiagFamily {
    /// `E###` — hard errors (parse / arity).
    Error,
    /// `W###` — lint warnings.
    Warning,
    /// `I###` — informational notes.
    Info,
    /// `S###` — shimmer (representation-change) notes.
    Shimmer,
    /// `T###` — taint / injection findings.
    Taint,
    /// `O###` — optimiser rewrites and constant-fold notes.
    Optimisation,
    /// `IRULE####` — iRules-dialect flow checks.
    IRule,
}

/// Error returned when a string is not a known [`DiagCode`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnknownDiagCode;

impl fmt::Display for UnknownDiagCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("unknown diagnostic code")
    }
}

macro_rules! diagnostic_codes {
    ($($variant:ident => $str:literal),+ $(,)?) => {
        /// A stable diagnostic-code identity. See the [module docs](self).
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum DiagCode {
            $(
                #[doc = concat!("The `", $str, "` diagnostic.")]
                $variant,
            )+
        }

        impl DiagCode {
            /// Every diagnostic code, in declaration order. Lets a test assert
            /// the emitted set and the documented table stay in lock-step.
            pub const ALL: &'static [DiagCode] = &[ $(DiagCode::$variant),+ ];

            /// The stable wire/UI spelling (e.g. `"W210"`).
            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self { $(DiagCode::$variant => $str,)+ }
            }
        }

        impl FromStr for DiagCode {
            type Err = UnknownDiagCode;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $($str => Ok(DiagCode::$variant),)+
                    _ => Err(UnknownDiagCode),
                }
            }
        }
    };
}

diagnostic_codes! {
    E001 => "E001",
    E002 => "E002",
    E003 => "E003",
    E004 => "E004",
    E100 => "E100",
    E101 => "E101",
    E102 => "E102",
    E103 => "E103",
    E200 => "E200",
    E201 => "E201",
    E202 => "E202",
    E203 => "E203",
    E204 => "E204",
    E205 => "E205",
    E206 => "E206",
    H300 => "H300",
    I230 => "I230",
    I231 => "I231",
    Irule1001 => "IRULE1001",
    Irule1002 => "IRULE1002",
    Irule1003 => "IRULE1003",
    Irule1004 => "IRULE1004",
    Irule1005 => "IRULE1005",
    Irule1006 => "IRULE1006",
    Irule1007 => "IRULE1007",
    Irule1008 => "IRULE1008",
    Irule1201 => "IRULE1201",
    Irule1202 => "IRULE1202",
    Irule2001 => "IRULE2001",
    Irule2002 => "IRULE2002",
    Irule2003 => "IRULE2003",
    Irule2101 => "IRULE2101",
    Irule3001 => "IRULE3001",
    Irule3002 => "IRULE3002",
    Irule3003 => "IRULE3003",
    Irule3004 => "IRULE3004",
    Irule3101 => "IRULE3101",
    Irule3102 => "IRULE3102",
    Irule3103 => "IRULE3103",
    Irule4001 => "IRULE4001",
    Irule4002 => "IRULE4002",
    Irule4003 => "IRULE4003",
    Irule4004 => "IRULE4004",
    Irule4005 => "IRULE4005",
    Irule5001 => "IRULE5001",
    Irule5002 => "IRULE5002",
    Irule5003 => "IRULE5003",
    Irule5004 => "IRULE5004",
    Irule5005 => "IRULE5005",
    Irule5006 => "IRULE5006",
    Irule5007 => "IRULE5007",
    Irule6001 => "IRULE6001",
    O100 => "O100",
    O101 => "O101",
    O102 => "O102",
    O103 => "O103",
    O104 => "O104",
    O105 => "O105",
    O106 => "O106",
    O107 => "O107",
    O108 => "O108",
    O109 => "O109",
    O110 => "O110",
    O111 => "O111",
    O112 => "O112",
    O113 => "O113",
    O114 => "O114",
    O115 => "O115",
    O116 => "O116",
    O117 => "O117",
    O118 => "O118",
    O119 => "O119",
    O120 => "O120",
    O121 => "O121",
    O122 => "O122",
    O123 => "O123",
    O124 => "O124",
    O125 => "O125",
    O126 => "O126",
    O127 => "O127",
    O128 => "O128",
    O129 => "O129",
    O130 => "O130",
    S100 => "S100",
    S101 => "S101",
    S102 => "S102",
    S110 => "S110",
    T100 => "T100",
    T101 => "T101",
    T102 => "T102",
    T103 => "T103",
    T104 => "T104",
    T105 => "T105",
    T106 => "T106",
    Tk1001 => "TK1001",
    Tk1002 => "TK1002",
    Tk1003 => "TK1003",
    W001 => "W001",
    W002 => "W002",
    W003 => "W003",
    W004 => "W004",
    W100 => "W100",
    W101 => "W101",
    W102 => "W102",
    W103 => "W103",
    W104 => "W104",
    W105 => "W105",
    W106 => "W106",
    W108 => "W108",
    W110 => "W110",
    W111 => "W111",
    W112 => "W112",
    W113 => "W113",
    W114 => "W114",
    W115 => "W115",
    W116 => "W116",
    W117 => "W117",
    W118 => "W118",
    W120 => "W120",
    W121 => "W121",
    W122 => "W122",
    W123 => "W123",
    W124 => "W124",
    W125 => "W125",
    W126 => "W126",
    W127 => "W127",
    W128 => "W128",
    W130 => "W130",
    W131 => "W131",
    W132 => "W132",
    W133 => "W133",
    W134 => "W134",
    W200 => "W200",
    W201 => "W201",
    W210 => "W210",
    W211 => "W211",
    W212 => "W212",
    W213 => "W213",
    W214 => "W214",
    W215 => "W215",
    W216 => "W216",
    W220 => "W220",
    W230 => "W230",
    W231 => "W231",
    W232 => "W232",
    W233 => "W233",
    W240 => "W240",
    W241 => "W241",
    W242 => "W242",
    W300 => "W300",
    W301 => "W301",
    W302 => "W302",
    W303 => "W303",
    W304 => "W304",
    W306 => "W306",
    W307 => "W307",
    W308 => "W308",
    W309 => "W309",
    W310 => "W310",
    W311 => "W311",
    W312 => "W312",
    W313 => "W313",
}

impl DiagCode {
    /// The family this code belongs to, from its letter prefix.
    #[must_use]
    pub const fn family(self) -> DiagFamily {
        let s = self.as_str().as_bytes();
        if s.len() >= 5
            && s[0] == b'I'
            && s[1] == b'R'
            && s[2] == b'U'
            && s[3] == b'L'
            && s[4] == b'E'
        {
            return DiagFamily::IRule;
        }
        // `TK###` Tk-toolkit lints share the `T` prefix with taint codes but are
        // plain warnings, so classify them before the single-byte match below.
        if s.len() >= 2 && s[0] == b'T' && s[1] == b'K' {
            return DiagFamily::Warning;
        }
        match s[0] {
            b'E' => DiagFamily::Error,
            b'I' => DiagFamily::Info,
            b'S' => DiagFamily::Shimmer,
            b'T' => DiagFamily::Taint,
            b'O' => DiagFamily::Optimisation,
            // `W###` (and any future prefix) fall through here; every variant's
            // spelling begins with one of the prefixes matched above.
            _ => DiagFamily::Warning,
        }
    }

    /// True for optimiser rewrite codes (`O###`) — replaces the scattered
    /// `code.starts_with('O')` checks the master optimiser switch keys on.
    #[must_use]
    pub const fn is_optimisation(self) -> bool {
        matches!(self.family(), DiagFamily::Optimisation)
    }

    /// True for hard-error codes (`E###`) — replaces the scattered
    /// `code.starts_with('E')` "did any error fire?" gates.
    #[must_use]
    pub const fn is_error(self) -> bool {
        matches!(self.family(), DiagFamily::Error)
    }
}

impl fmt::Display for DiagCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_and_from_str_round_trip() {
        for &code in DiagCode::ALL {
            assert_eq!(DiagCode::from_str(code.as_str()), Ok(code));
        }
    }

    #[test]
    fn unknown_code_is_rejected() {
        assert_eq!(DiagCode::from_str("W999"), Err(UnknownDiagCode));
        assert_eq!(DiagCode::from_str("nonsense"), Err(UnknownDiagCode));
    }

    #[test]
    fn strings_are_unique() {
        let mut seen: heapless_set::Set = heapless_set::Set::new();
        for &code in DiagCode::ALL {
            assert!(
                seen.insert(code.as_str()),
                "duplicate spelling {}",
                code.as_str()
            );
        }
    }

    #[test]
    fn families_are_classified() {
        assert_eq!(DiagCode::W210.family(), DiagFamily::Warning);
        assert_eq!(DiagCode::E001.family(), DiagFamily::Error);
        assert_eq!(DiagCode::O100.family(), DiagFamily::Optimisation);
        assert_eq!(DiagCode::T100.family(), DiagFamily::Taint);
        assert_eq!(DiagCode::S100.family(), DiagFamily::Shimmer);
        assert_eq!(DiagCode::I230.family(), DiagFamily::Info);
        assert_eq!(DiagCode::Irule3102.family(), DiagFamily::IRule);
        assert!(DiagCode::O129.is_optimisation());
        assert!(!DiagCode::W210.is_optimisation());
    }

    /// Tiny `no_std` set for the uniqueness test (avoids pulling in std).
    mod heapless_set {
        pub struct Set {
            items: [&'static str; 256],
            len: usize,
        }
        impl Set {
            pub const fn new() -> Self {
                Self {
                    items: [""; 256],
                    len: 0,
                }
            }
            pub fn insert(&mut self, s: &'static str) -> bool {
                let mut i = 0;
                while i < self.len {
                    if self.items[i] == s {
                        return false;
                    }
                    i += 1;
                }
                self.items[self.len] = s;
                self.len += 1;
                true
            }
        }
    }
}
