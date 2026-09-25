# 0005. Read Claude plan limits through the Claude Code status line

- **Status:** Accepted
- **Date:** 2026-09-24

## Context

The first AI metric espia shows is how much of a Claude Pro or Max
subscription's usage limits has been consumed: the rolling **5-hour** window
and the **weekly** window, each with its reset time.

Anthropic does not offer a public, documented API for subscription limits.
Two sources exist:

1. **Claude Code status line input (documented).** Claude Code runs a
   user-configured [status line](https://code.claude.com/docs/en/statusline)
   command and passes it a JSON object on stdin. For Pro and Max subscribers,
   that object includes:

   ```json
   "rate_limits": {
     "five_hour": { "used_percentage": 23.5, "resets_at": 1738425600 },
     "seven_day": { "used_percentage": 41.2, "resets_at": 1738857600 }
   }
   ```

   `resets_at` is in Unix epoch **seconds**. Each window may be absent, and
   the object only appears after the first API response in a session.

2. **`GET /api/oauth/usage` (undocumented).** The endpoint Claude Code itself
   calls, authenticated with the user's Claude Code OAuth token. It returns
   the same windows at any time, but it is not a public API, it is
   [heavily rate-limited](https://github.com/anthropics/claude-code/issues/31637),
   and using it means reading the user's OAuth credentials from Claude Code's
   credential store.

## Decision

Use the **status line** as the source (`claude_code_statusline`).

- The agent ships a **bridge** subcommand (for example,
  `espia-agent claude-statusline`). It is part of the agent binary, so it
  runs the same way on Windows, macOS, and Linux without `jq`, `curl`, or a
  POSIX shell.
- From its settings UI, and only with the user's consent, the agent sets the
  bridge as `statusLine.command` in the user's Claude Code settings.
- On every run, the bridge:
  1. reads the JSON from stdin;
  2. atomically writes `rate_limits` and a timestamp to a state file in the
     agent's data directory, which the agent watches;
  3. runs the user's **previous** status line command, if any, with the same
     stdin, and prints its output, so the user's existing status line keeps
     working.
- Uninstalling restores the previous `statusLine` setting exactly.

The undocumented OAuth endpoint is **not** used.

## Consequences

- Relies only on documented behavior and never touches the user's
  credentials.
- Data is only as fresh as the last Claude Code activity. When Claude Code is
  not running, the device shows the last known values with their age. Once a
  window's `resets_at` passes, the agent knows that window has reset.
- The values are reported by Anthropic's servers for the whole account, but
  are only refreshed when Claude Code gets a response. Usage in other Claude
  apps shows up the next time Claude Code is used.
- Users of claude.ai who never use Claude Code get no plan-limit data from
  this source.
- Modifying `~/.claude/settings.json` is invasive. The change must be opt-in,
  clearly explained, reversible, and must preserve any existing status line.
- If the user had no status line before, the bridge must print something
  useful (for example, the model name and the 5-hour usage). A configured
  status line replaces some of Claude Code's footer hints.
- If several Claude Code sessions run at the same time, the most recent write
  wins. That's fine for a single account; several accounts on one machine are
  not supported.

## Implementation notes

- `connect` is idempotent: calling it while already connected does not
  overwrite the backup with the agent's own bridge command.
- The bridge treats the previous `statusLine.command` as fully opaque — it
  never inspects what it does, only that running it with the same stdin
  produces the right output. This matters in practice: a status line is
  sometimes owned by another tool the user has installed, not something they
  wrote by hand, and the bridge chains to it correctly either way.
- Rewriting `settings.json` uses `serde_json`'s `preserve_order` feature, so
  unrelated keys keep their original order instead of being alphabetized.
- Implemented in `agent/src-tauri/src/providers/claude.rs`, with tests that
  point `$HOME` at a scratch directory to exercise the real
  read-modify-write settings.json logic without touching a real user's
  files.

## Alternatives considered

- **Undocumented OAuth usage endpoint** — works without Claude Code
  activity, but relies on a private API that may change or break, is
  aggressively rate-limited, and requires handling the user's OAuth token.
  We may revisit it as an opt-in, experimental source if an official API
  becomes available.
- **Estimating limits from local token logs** — Anthropic does not publish
  exact limits, so estimates would be misleading.
