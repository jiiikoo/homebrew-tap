# Security Policy

## Reporting a vulnerability

Please report security issues **privately** — open a private security advisory on the
repository (GitHub → Security → Report a vulnerability), or contact the maintainer directly,
rather than filing a public issue. We'll acknowledge and work on a fix before disclosure.

> Maintainers: add a concrete contact (email / advisory link) here before publishing.

## Threat model (summary)

`sshelf` can store SSH passwords so it can auto-supply them. **Prefer SSH keys / an agent
wherever possible** — stored passwords are the least secure option offered.

**Where secrets live:** the OS keyring (macOS Keychain, Linux Secret Service via a pure-Rust
client) by default; or, if `SSHELF_VAULT_PASSPHRASE` is set, an `age`-encrypted `vault.age`
(scrypt + ChaCha20-Poly1305) for headless/automation use. Secrets are keyed by host id and are
**never** written to `hosts.toml`, logs, shell history, or process arguments.

**How the secret reaches ssh:** via `SSH_ASKPASS` — `sshelf` is re-invoked by `ssh` and prints
the secret on stdout. It matches the *shape* of OpenSSH's standard prompts (a login password
`…password:` or a key passphrase `Enter passphrase for key …`) and declines host-key
confirmations, OTP codes, and arbitrary server text — so a keyboard-interactive server cannot
phish the stored secret by merely mentioning "password". (`-o StrictHostKeyChecking=accept-new` keeps the
first-connect host-key prompt out of the helper while still verifying known hosts.)

### Protected against
- Plaintext-on-disk exposure (secrets are in the keyring or encrypted at rest).
- Process-listing / argv leakage (no `sshpass -p`).
- Shell-history leakage; `hosts.toml` is safe to share/commit (no secrets).
- Config corruption (atomic writes).

### NOT protected against (out of scope)
- A root/admin attacker or malware on your machine (can read process memory / the keyring).
- Keyloggers (can capture a typed vault passphrase).
- A compromised OS keyring daemon.
- Physical theft without full-disk encryption.
- Unencrypted backups/cloud-sync of `vault.age` (it's encrypted, but treat it as sensitive).

Assumption: `sshelf` runs on a machine you control and trust. There is **no recovery** if you
forget the vault passphrase.

## Platform notes
- **macOS, unsigned builds:** the re-invoked askpass helper reads Keychain as a *separate*
  process; because Keychain ACLs are tied to code signature, an unsigned dev build may prompt
  for Keychain access on each connect. Use a signed release build, or the vault, to avoid this.
- **Windows** is not supported in v1.
