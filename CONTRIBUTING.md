# Contributing to espia

Thanks for your interest in espia! This document explains how to propose
changes and what we expect from contributions.

## Ground rules

- **English only.** Code, comments, commit messages, issues, pull requests,
  and documentation are written in English.
- Be respectful. Everyone participating is expected to follow the
  [Code of Conduct](CODE_OF_CONDUCT.md).
- For anything larger than a small fix, open an issue first so we can agree on
  the approach before you invest time in it.

## Project structure

espia is a monorepo with three main parts:

- `agent/` — desktop agent (Windows, macOS, Linux)
- `firmware/` — ESP32 firmware
- `protocol/` — the contract between them

Changes that touch the protocol must update
[`protocol/README.md`](protocol/README.md) and its examples in the same pull
request. Breaking protocol changes require a version bump.

## Architecture decisions

Significant technical decisions are recorded as
[Architecture Decision Records](docs/adr/). If your change introduces a new
dependency, a new component, or reverses an earlier decision, add or update
an ADR using [the template](docs/adr/0000-template.md).

## Commit messages

We follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <short summary>

<body explaining why the change is needed>
```

- **Types:** `feat`, `fix`, `docs`, `refactor`, `perf`, `test`, `build`,
  `ci`, `chore`
- **Scopes:** `agent`, `firmware`, `protocol`, `docs`, `repo`

Example:

```
feat(firmware): rediscover the agent after the connection drops

The agent's IP address can change after a DHCP renewal. Instead of retrying
the last known address forever, fall back to discovery after three failed
reconnection attempts.
```

The body should explain **why** the change exists, not only what it does.

## Pull requests

1. Fork the repository and create a branch from `main`
   (for example `feat/udp-discovery`).
2. Keep pull requests focused on a single concern.
3. Update documentation and the [changelog](CHANGELOG.md) when behavior changes.
4. Make sure builds and tests pass locally.
5. Fill in the pull request template.

## Reporting bugs and requesting features

Use the [issue templates](.github/ISSUE_TEMPLATE/). For security issues,
follow the [security policy](SECURITY.md) instead of opening a public issue.

## License

By contributing, you agree that your contributions will be licensed under the
[MIT License](LICENSE).
