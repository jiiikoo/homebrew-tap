# `sshelf` documentation

`sshelf` is a TUI for managing and connecting to SSH hosts. It keeps its own host database
and generates the correct `ssh` command for each node — it never edits `~/.ssh/config`.

> **Docs-in-sync rule:** every code/behavior change updates the relevant doc here in the same
> change, and appends to [`progress.md`](./progress.md). See [`../CONTRIBUTING.md`](../CONTRIBUTING.md).

## Contents

| Doc | What it covers |
|---|---|
| [progress.md](./progress.md) | Living log — current milestone, what changed, what's next. **Start here.** |
| [architecture.md](./architecture.md) | How the pieces fit: launcher/`exec()` model, askpass flow, secret store, data flow. |
| [structure.md](./structure.md) | Module/file map and responsibilities. |
| [data-model.md](./data-model.md) | `Host` schema, config/state files, on-disk locations & formats. |
| [ssh-command.md](./ssh-command.md) | Flag mapping (`-i`/`-p`/`-J`/extra-args) and the `SSH_ASKPASS` password mechanism. |
| [ux.md](./ux.md) | Screens, keybindings, wizard flow, theming. |
| [decisions.md](./decisions.md) | Decision log (ADR-style) with rationale. |
| [security.md](./security.md) | Threat model for stored secrets (mirrors the shipped `SECURITY.md`). |
| [packaging.md](./packaging.md) | Shipping to Homebrew, Debian/Ubuntu (`.deb`/apt), and crates.io — multi-arch (x86 + arm). |

## Quick orientation

- **What it is:** a fast, atuin-style fuzzy launcher for SSH. Save a host once, connect with `Enter`.
- **What it is NOT:** it does not edit `~/.ssh/config`, is not a terminal emulator, and does
  not proxy/tunnel traffic itself — it builds the `ssh` invocation and hands the terminal to `ssh`.
- **Platforms:** macOS + Linux (v1). **Toolchain:** Rust 1.88+.
- **Status:** see [progress.md](./progress.md).
