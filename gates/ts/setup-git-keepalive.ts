#!/usr/bin/env bun
//
// Bake the SSH keepalive into this clone's git config.
//
// Why this exists: git opens the transport connection to the remote BEFORE it
// runs the pre-push hook, then sits idle for however long the gates take. On a
// multi-minute hook GitHub drops the idle SSH channel, and the push dies with
//
//     Connection to github.com closed by remote host.
//     fatal: the remote end hung up unexpectedly
//     (exit 141 / SIGPIPE)
//
// — AFTER every gate has already printed ✓. That reads as "the gates failed"
// when nothing failed at all, and has cost multiple sessions a long
// misdiagnosis. `ServerAliveInterval` makes the client send traffic every 20s
// so the channel is never idle; `ServerAliveCountMax=60` tolerates a ~20 min
// hook without the client giving up on the server either.
//
// Unified gate caching makes hooks short enough that this should never
// trigger — but the cost is zero and the failure it prevents is a 20-minute
// misdiagnosis, so it stays as a backstop.
//
// Delivery: run from `prepare` in package.json, i.e. every `bun install`, which
// is the first thing a fresh clone does — and again from `.husky/pre-push` as a
// belt-and-braces backstop.
//
// WHY TypeScript and not bash (the hook glue next door) or Rust (the default
// for tooling here): `prepare` runs inside `bun install` on EVERY platform,
// including the Windows release runner, where bun's shell cannot execute a
// `.sh` — `bun: command not found: ./setup-git-keepalive.sh`
// failed the whole install and killed the Windows release job. Rust is
// impractical here because prepare runs before any toolchain is guaranteed and
// must not compile anything; bun is guaranteed present because it IS the
// package manager running this script. It is ~10 lines of `git config` glue.
//
// Fails OPEN everywhere: no git, no repo, or an existing custom sshCommand and
// we simply do nothing. Nobody is ever blocked by this script, on any platform.

const KEEPALIVE = "ssh -o ServerAliveInterval=20 -o ServerAliveCountMax=60"

/** Run git, returning trimmed stdout — or null when git is absent or errors. */
function git(...args: string[]): string | null {
  try {
    const proc = Bun.spawnSync(["git", ...args], { stdout: "pipe", stderr: "ignore" })
    if (proc.exitCode !== 0) return null
    return proc.stdout.toString().trim()
  } catch {
    // No git on PATH (or spawn refused) — nothing to configure.
    return null
  }
}

// Not a git repo (or no git at all) → nothing to do.
if (git("rev-parse", "--git-dir") === null) process.exit(0)

// Respect an existing setting — a contributor's 1Password/ssh-agent wrapper,
// or a value they set deliberately, must win over ours.
if (git("config", "--local", "--get", "core.sshCommand")) process.exit(0)

// `--local` writes to the COMMON config ($GIT_COMMON_DIR/config), so a single
// write covers the primary checkout and every `git worktree` hanging off it.
if (git("config", "--local", "core.sshCommand", KEEPALIVE) !== null) {
  console.log("  ✓ git core.sshCommand set to keep the push connection alive during pre-push")
}

process.exit(0)
