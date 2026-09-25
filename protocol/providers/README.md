# Providers

Each provider documents the shape of its `data` object in the
[`provider`](../README.md#52-provider-agent--device) message.

| Provider           | Status | Document               |
| ------------------ | ------ | ---------------------- |
| `claude`           | Draft  | [claude/](claude/README.md) |

## Adding a provider

1. Pick a short, lowercase identifier (for example, `openai`).
2. Create `<identifier>/README.md` describing its data sources and `data` fields,
   following the [conventions](../README.md#6-conventions).
3. Add an example under [`../examples/`](../examples/).
