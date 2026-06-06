//! The host data model and the on-disk `hosts.toml` shape.
//!
//! Invariant: **no secrets live here.** `auth = "password"` only records that a host
//! uses password auth; the actual password is stored in the keyring/vault (see `secrets`).

use serde::{Deserialize, Serialize};

/// How a host authenticates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AuthMethod {
    /// Public-key auth with one or more identity files (`-i`).
    Key,
    /// Password auth; the secret is auto-supplied via the askpass helper.
    Password,
    /// Rely on a running ssh-agent (the default).
    #[default]
    Agent,
}

impl AuthMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            AuthMethod::Key => "key",
            AuthMethod::Password => "password",
            AuthMethod::Agent => "agent",
        }
    }
}

/// A single saved host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Host {
    /// Stable unique id (ULID). Keys both the secret store and frecency state, so a host
    /// can be renamed without losing its password or usage history.
    pub id: String,
    /// Display alias (what you search and see in the list).
    pub name: String,
    /// IP address or DNS name. Required.
    pub hostname: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,

    #[serde(default)]
    pub auth: AuthMethod,

    /// Identity files for `auth = key` (repeatable `-i`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub identity_files: Vec<String>,
    /// ProxyJump chain (`-J a,b,c`). Key/agent auth only in v1.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub jump_hosts: Vec<String>,
    /// Free-form tags for filtering/grouping.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Raw extra args appended verbatim (shlex-split). Escape hatch for anything unmodeled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra_args: Option<String>,
}

impl Host {
    /// Create a new host with a freshly generated id and sensible defaults.
    pub fn new(name: impl Into<String>, hostname: impl Into<String>) -> Self {
        Host {
            id: ulid::Ulid::new().to_string(),
            name: name.into(),
            hostname: hostname.into(),
            user: None,
            port: None,
            auth: AuthMethod::default(),
            identity_files: Vec::new(),
            jump_hosts: Vec::new(),
            tags: Vec::new(),
            extra_args: None,
        }
    }

    /// The user to connect as: the stored user, else `$USER`, else `"root"`.
    pub fn effective_user(&self) -> String {
        self.user
            .clone()
            .or_else(|| std::env::var("USER").ok())
            .unwrap_or_else(|| "root".to_string())
    }

    pub fn port_or_default(&self) -> u16 {
        self.port.unwrap_or(22)
    }

    /// `user@host:port` summary used in the list and previews.
    pub fn endpoint(&self) -> String {
        format!(
            "{}@{}:{}",
            self.effective_user(),
            self.hostname,
            self.port_or_default()
        )
    }

    /// The haystack string used for fuzzy matching (name + endpoint + tags).
    pub fn search_haystack(&self) -> String {
        let mut s = format!("{} {}", self.name, self.endpoint());
        if !self.tags.is_empty() {
            s.push(' ');
            s.push_str(&self.tags.join(" "));
        }
        s
    }
}

/// The whole `hosts.toml` file. `format_version` is declared first so it serializes
/// before the `[[host]]` array (TOML requires scalars before array-of-tables).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostsFile {
    pub format_version: u32,
    #[serde(default, rename = "host", skip_serializing_if = "Vec::is_empty")]
    pub hosts: Vec<Host>,
}

pub const CURRENT_FORMAT_VERSION: u32 = 1;

impl Default for HostsFile {
    fn default() -> Self {
        HostsFile {
            format_version: CURRENT_FORMAT_VERSION,
            hosts: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sane() {
        let h = Host::new("web", "10.0.0.1");
        assert_eq!(h.port_or_default(), 22);
        assert_eq!(h.auth, AuthMethod::Agent);
        assert!(!h.id.is_empty());
    }

    #[test]
    fn effective_user_prefers_explicit() {
        let mut h = Host::new("web", "10.0.0.1");
        h.user = Some("deploy".into());
        assert_eq!(h.effective_user(), "deploy");
    }

    #[test]
    fn auth_serializes_lowercase() {
        let json = serde_json::to_string(&AuthMethod::Password).unwrap();
        assert_eq!(json, "\"password\"");
    }
}
