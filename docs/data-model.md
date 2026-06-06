# Data model & on-disk layout

## File locations

Paths resolve via the `etcetera` **base strategy** (XDG everywhere — `~/.config/sshelf` on
both macOS *and* Linux, honoring `XDG_CONFIG_HOME`/`XDG_DATA_HOME` when set). This keeps
config hand-editable instead of buried in macOS `~/Library`.

| File | Location (default) | Owner | Purpose |
|---|---|---|---|
| `hosts.toml` | `~/.config/sshelf/hosts.toml` | user | The host database. Human-editable. |
| `config.toml` | `~/.config/sshelf/config.toml` | user | Preferences (theme, `decay_rate`, sort, keybinds). |
| `state.json` | `~/.local/share/sshelf/state.json` | app | Frecency counters, keyed by host id. Churns; not for hand-editing. |
| `vault.age` | `~/.local/share/sshelf/vault.age` | app | **Fallback** encrypted secret store (only when no OS keyring). Mode `0600`. |

Directories are created on first run (`0700`). **Secrets are never written to `hosts.toml`.**

## `Host` schema (`hosts.toml`)

```toml
format_version = 1            # top-level; for future migrations

[[host]]
id        = "01J…"            # stable unique id (e.g. ULID/UUID); keys secrets & frecency
name      = "prod-db"         # display alias (what you search/see)
hostname  = "10.25.25.25"     # IP or DNS name           (required)
user      = "mike"            # optional; default = $USER at connect time
port      = 22                # optional; default 22
auth      = "key"             # "key" | "password" | "agent"
identity_files = ["~/.ssh/infra-key"]   # for auth="key"; repeatable (-i per entry)
jump_hosts = ["bastion.example.com"]    # ProxyJump chain; key/agent auth only in v1
tags      = ["prod", "db"]    # for filtering/grouping
extra_args = "-o ServerAliveInterval=30"  # raw, shlex-split, appended verbatim
# NOTE: no password field — ever. auth="password" means "look up the secret by id".
```

Notes:
- Optional fields use `Option<T>` in Rust with `#[serde(skip_serializing_if = "Option::is_none")]`
  so the TOML stays clean; new fields use `#[serde(default)]` for backward compatibility.
- `identity_files` / `jump_hosts` / `tags` are `Vec<String>` (empty = absent).
- `format_version` lets us migrate the schema later without breaking older files.

## Frecency state (`state.json`)

```json
{
  "01J…": { "use_count": 12, "last_used": "2026-06-05T09:30:00Z" }
}
```

- Keyed by host `id` (so renaming a host in `hosts.toml` keeps its history).
- Updated **before** `exec()` on connect: `use_count += 1`, `last_used = now`.
- Kept separate from `hosts.toml` so the user-owned host file stays stable and diff-friendly.
- Score: `use_count * exp(-decay_rate * days_since_last_used)` (`decay_rate` default `0.2`).
  See [`ux.md`](./ux.md) for how it combines with fuzzy ranking.

## Secrets

Stored in the OS keyring (service `sshelf`, account = host `id`) or, as a fallback on
headless systems, in `vault.age`. Either way the key is the host `id`. Full model and threat
analysis in [`security.md`](./security.md).

## Atomic writes

All persistent writes use temp-file + `rename()` (atomic on Unix) so a crash mid-write never
corrupts `hosts.toml` / `config.toml` / `state.json`. Single-process tool → no file locking needed.
