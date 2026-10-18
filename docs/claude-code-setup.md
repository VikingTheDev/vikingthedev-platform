# Claude Code setup

This document explains how `.claude/settings.json` keeps secrets and credentials away from Claude, and lists the checks to run whenever the setup changes.

## What the settings enforce

### The shell sandbox (the real boundary)

The sandbox applies to every command Claude runs in its shell.

- **Reads:** all of `~/` is denied, except:
  - the project directory;
  - the toolchain folders: `~/.cargo`, `~/.rustup`, fnm's Node installs, the corepack cache, and the pnpm store and cache.

  Inside those allowed folders, the cargo credentials files and fnm's global `npmrc` stay denied.
- **Project files:** reads of `.env*` and `*.pem` anywhere in the project are denied.
- **Writes:** writes to `.claude/` and `scripts/with-secrets` are denied.
- **No escape hatch:** `allowUnsandboxedCommands: false` means a command can't fall back to running outside the sandbox.
- **Startup:** `failIfUnavailable: true` means Claude Code won't start if the sandbox can't run.
- **Network:** limited to the npm and crates.io registries, GitHub and GHCR.
- **Environment:** `credentials.envVars` removes known secret variables from every sandboxed command, including tokens for Coolify, Cloudflare, R2, OAuth, GitHub, npm and cargo.

### Rules for Claude's file tools

Claude's file tools (Read, Edit, Write) run outside the sandbox, so they have their own deny rules.

- **Read deny rules** cover:
  - `.env*` and `*.pem`;
  - `~/.config/vikingthedev`;
  - `~/.ssh`, `~/.aws`, `~/.docker` and `~/.config/gh`;
  - the cargo credentials files, `~/.npmrc` and fnm's global `npmrc`;
  - `~/.git-credentials` and `~/.netrc`;
  - `/proc`.
- **Edit deny rules** protect `.claude/settings.json`, `.claude/settings.local.json`, `~/.claude/settings.json` and `scripts/with-secrets`.
  - Not protected: `~/.claude/settings.local.json`, `CLAUDE.md`, `.claude/rules/*.md` and `.claude/agents/*.md`.
  - The sandbox's `denyWrite` on `.claude/` covers shell commands only. Claude's Edit tool can still change the rule files, so changes to them rely on review.

### Limits

- **The script deny is weak.** The deny on running `scripts/with-secrets` is only a speed bump, because `bash scripts/with-secrets` gets around it. The sandbox's read denies are what actually protect the secrets.
- **`!` commands bypass the sandbox.** Commands you type with the `!` prefix run unsandboxed, and their output enters the conversation. Never load secrets that way.
- **No pushing.** Claude can't push to GitHub or use `gh`, because `~/.ssh` and `~/.config/gh` are denied. The human pushes.
- **Start Claude Code from a clean shell.** Never launch it from a shell with secrets exported. The env scrubbing only covers the variable names it lists.
- **Review is guidance, not enforcement.** To make the pre-commit review mandatory, add a `PreToolUse` hook on `git commit` that blocks the commit unless a review marker exists for the current staged-tree hash.

## Requirements

- The sandbox needs `bubblewrap` and `socat` installed.
- Git identity is set in the repo's local `.git/config`, because the sandbox denies `~/.gitconfig`.

## Checks to run when the setup or settings change

1. **Secret files:** put a sentinel value in a dummy `.env`. Both Claude's Read tool and a sandboxed `cat .env` must fail.
2. **Other processes:** a sandboxed `cat /proc/<pid>/environ` of a process you started yourself must fail.
3. **Toolchain:** `pnpm install`, `cargo build` and `docker compose -f compose.dev.yaml up` must work inside the sandbox.
   - The toolchain `allowRead` paths are best guesses.
   - Docker socket access from the sandbox is untested.
4. **Advisor:** the advisor is off unless you enable it with `/advisor`. The project settings don't set `advisorModel`.
