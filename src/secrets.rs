//! Secret storage router.
//!
//! - If `SSHELF_VAULT_PASSPHRASE` is set → use the age-encrypted [`crate::vault`] (good for
//!   headless Linux / automation, and deterministic for testing).
//! - Otherwise → use the OS keyring (macOS Keychain, Linux Secret Service, Windows
//!   Credential Manager).
//!
//! Secrets are keyed by the stable host id. See `docs/security.md` for the threat model.

use std::path::Path;

use anyhow::{Context, Result};

const SERVICE: &str = "sshelf";
pub const VAULT_PASS_ENV: &str = "SSHELF_VAULT_PASSPHRASE";

fn vault_passphrase() -> Option<String> {
    std::env::var(VAULT_PASS_ENV).ok().filter(|s| !s.is_empty())
}

fn keyring_entry(id: &str) -> Result<keyring::Entry> {
    keyring::Entry::new(SERVICE, id).context("opening keyring entry")
}

pub fn store_password(vault_path: &Path, id: &str, password: &str) -> Result<()> {
    if let Some(pass) = vault_passphrase() {
        crate::vault::store(vault_path, &pass, id, password)
    } else {
        keyring_entry(id)?
            .set_password(password)
            .context("storing password in OS keyring")
    }
}

pub fn get_password(vault_path: &Path, id: &str) -> Result<Option<String>> {
    if let Some(pass) = vault_passphrase() {
        crate::vault::get(vault_path, &pass, id)
    } else {
        match keyring_entry(id)?.get_password() {
            Ok(p) => Ok(Some(p)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e).context("reading password from OS keyring"),
        }
    }
}

pub fn delete_password(vault_path: &Path, id: &str) -> Result<()> {
    if let Some(pass) = vault_passphrase() {
        crate::vault::delete(vault_path, &pass, id)
    } else {
        match keyring_entry(id)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e).context("deleting password from OS keyring"),
        }
    }
}
