//! Declarative capabilities — the serde-facing vocabulary a build script or
//! operator hook uses to request authority, and that an operator policy uses to
//! grant or deny it.
//!
//! A package's build script declares the capabilities it needs; the sandbox
//! grants the *minimum* of what was requested and what policy allows, never
//! more. This mirrors the capability-declaration model used by tools such as
//! Deno and Bun, where code states its needs up front instead of inheriting the
//! caller's full authority.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::Profile;

/// A single declared capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Capability {
    /// Permission to use the network.
    Network,
    /// Permission to read a path subtree.
    FsRead(PathBuf),
    /// Permission to write a path subtree.
    FsWrite(PathBuf),
    /// Permission to pass a named environment variable through.
    Env(String),
}

/// A declared capability set, e.g. parsed from a manifest `build` directive or a
/// hook definition in policy TOML.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilitySet {
    #[serde(default)]
    pub network: bool,
    #[serde(default)]
    pub fs_read: Vec<PathBuf>,
    #[serde(default)]
    pub fs_write: Vec<PathBuf>,
    #[serde(default)]
    pub env: Vec<String>,
}

impl CapabilitySet {
    /// Fold this declared set into a [`Profile`], widening the profile's
    /// requested authority. The policy floor is applied later, in
    /// [`crate::run`].
    #[must_use]
    pub fn apply_to(&self, mut profile: Profile) -> Profile {
        if self.network {
            profile.allow_network = true;
        }
        profile.fs_read.extend(self.fs_read.iter().cloned());
        profile.fs_write.extend(self.fs_write.iter().cloned());
        profile.env_passthrough.extend(self.env.iter().cloned());
        profile
    }

    /// Enumerate the set as individual [`Capability`] values (for display).
    #[must_use]
    pub fn to_capabilities(&self) -> Vec<Capability> {
        let mut caps = Vec::new();
        if self.network {
            caps.push(Capability::Network);
        }
        caps.extend(self.fs_read.iter().cloned().map(Capability::FsRead));
        caps.extend(self.fs_write.iter().cloned().map(Capability::FsWrite));
        caps.extend(self.env.iter().cloned().map(Capability::Env));
        caps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_widens_profile() {
        let caps = CapabilitySet {
            network: true,
            fs_read: vec![PathBuf::from("/pkg")],
            fs_write: vec![PathBuf::from("/out")],
            env: vec!["TCLLIBPATH".into()],
        };
        let profile = caps.apply_to(Profile::new("build", "/bin/sh", "/pkg"));
        assert!(profile.allow_network);
        assert_eq!(profile.fs_read, vec![PathBuf::from("/pkg")]);
        assert_eq!(profile.fs_write, vec![PathBuf::from("/out")]);
        assert!(profile.env_passthrough.contains(&"TCLLIBPATH".to_string()));
    }

    #[test]
    fn to_capabilities_enumerates() {
        let caps = CapabilitySet {
            network: true,
            fs_read: vec![PathBuf::from("/a")],
            ..CapabilitySet::default()
        };
        let list = caps.to_capabilities();
        assert!(list.contains(&Capability::Network));
        assert!(list.contains(&Capability::FsRead(PathBuf::from("/a"))));
    }
}
