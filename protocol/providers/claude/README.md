# Provider: `claude`

- **Status:** Draft

Usage data for [Claude](https://claude.com) by Anthropic.

## Data sources

| `source`                 | Data                     | Status  | Credentials   |
| ------------------------ | ------------------------ | ------- | ------------- |
| `claude_code_statusline` | Subscription plan limits | Planned | None          |
| `claude_code_logs`       | Token usage and cost     | Future  | None          |
| `anthropic_api`          | Organization usage/cost  | Future  | Admin API key |

### `claude_code_statusline`

Plan limits for Claude Pro and Max subscriptions, captured from the JSON that
Claude Code passes to its status line command. See
[ADR 0005](../../../docs/adr/0005-claude-plan-limits-source.md) for how the agent
collects it.

## Fields

### `plan_limits`

Present when `source` is `claude_code_statusline`.

| Path                             | Type    | Description                                            |
| -------------------------------- | ------- | ------------------------------------------------------ |
| `plan_limits.five_hour.used_pct` | number  | Share of the rolling 5-hour limit used (0–100)         |
| `plan_limits.five_hour.resets_at`| integer | When the 5-hour window resets                          |
| `plan_limits.seven_day.used_pct` | number  | Share of the weekly limit used (0–100)                 |
| `plan_limits.seven_day.resets_at`| integer | When the weekly window resets                          |
| `plan_limits.updated_at`         | integer | When the agent last received these values from Claude Code |

- Each window may be omitted if Claude Code did not report it.
- If a window's `resets_at` is in the past, the agent reports that window
  with `used_pct` set to `0` and `resets_at` set to `null`: the window has
  reset, but the next reset time is unknown until new data arrives.
- `updated_at` lets the device show how old the data is (for example,
  "updated 2 h ago").

See [`provider-claude.json`](../../examples/provider-claude.json).

### `usage` (future)

Token counts and estimated cost from Claude Code session logs. To be
specified.
