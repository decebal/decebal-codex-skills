---
title: Environment Debugging
tags: env-vars, secrets, tls, certificates, dns, path
---

# Environment Debugging

"Works on my machine" is an environment bug until proven otherwise. The environment
is state too — read it, do not assume its defaults.

## Environment variables

Three distinct states that look alike and behave differently:
- **Set but not exported** — visible to the shell, invisible to a child process. `FOO=bar`
  without `export` means your program never sees it.
- **Exported and non-empty** — the good case.
- **Exported but empty** — present, so a `if [ -z "$FOO" ]` guard may pass or a "default
  if unset" (`${FOO:-default}`) may NOT trigger (empty is set). An empty string is the
  sneakiest of the three.

```bash
# is the child process actually getting it? read the process environment, not your shell
ps -eww -o pid,command | grep <proc>          # find the pid
tr '\0' '\n' < /proc/<pid>/environ | grep FOO # Linux: the process's real env
# does the shell export it (vs merely set it)?
export -p | grep FOO
```

Read the value from the RUNNING process, not from your interactive shell — they can
differ (different profile, a supervisor that strips env, a container with its own
environment).

## Secrets

- A secret that is present but WRONG fails identically to one that is absent (401/403).
  Confirm which: is the variable set at all, and does its value match the expected
  fingerprint (length, prefix, a non-secret hash) — never print the value itself.
- Secrets injected at deploy time are often absent in a fresh shell, a new worktree, or
  a locally-run process. "It's in CI" does not mean it is in the environment you are
  running in now.
- A rotated secret with a stale copy cached in a running process fails only after
  restart clears it — or only after it does NOT. Check when the process last started.

## Certificates / TLS chains

- `x509: certificate signed by unknown authority` → the client does not trust the CA
  that signed the server's cert. Missing intermediate in the chain, or a private CA not
  in the trust store.
- Inspect what the server actually serves and whether the chain is complete:

```bash
openssl s_client -connect host:443 -servername host </dev/null 2>/dev/null | openssl x509 -noout -dates -subject -issuer
```

- Check **expiry** (`-dates`), **hostname/SNI match** (`-subject` and SANs vs the name
  you connect as), and that the server sends **intermediates**, not just the leaf. A cert
  valid in the browser but failing in code often means the browser had the intermediate
  cached and your client does not.

## DNS

- Resolve the name explicitly before blaming the app: `dig +short host` /
  `nslookup host`. No answer → DNS; an answer pointing at the wrong IP → stale record or
  wrong resolver (`/etc/resolv.conf`, a VPN split-horizon, a container's DNS).
- `/etc/hosts` overrides DNS — a leftover entry sends you to the wrong place while `dig`
  (which ignores hosts) looks fine. Check both.

## PATH

- "command not found" or the WRONG binary running is a PATH bug. `which -a <cmd>` shows
  every match in order; the first wins. A tool that behaves differently than expected may
  be a different version earlier on PATH.
- A cron job, systemd unit, or CI step has a MINIMAL PATH, not your interactive one — a
  command that works in your terminal fails there because its directory is not on the
  reduced PATH. Use absolute paths or set PATH explicitly in those contexts.

See [../../../rules/evidence-discipline.md](../../../rules/evidence-discipline.md) — read the
actual environment state; an absence at the source has an innocent and an alarming
reading, and only the artifact tells you which.
