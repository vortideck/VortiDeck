# Contributing to VortiDeck

Thanks for helping. VortiDeck is maintained by Alexi Pawelec, so reviews can take a few days.

## Before you start

- For anything bigger than a small fix, open an issue first so we can agree on the approach.
- Security problems: do not open a public issue. Follow [SECURITY.md](SECURITY.md).

## Development setup

See "Building from source" in the [README](README.md). Before opening a pull request, run the checks for the area you changed:

```bash
pnpm --filter vortideck-desktop test
```

```bash
pnpm check:desktop:rust
```

```bash
pnpm check:lint-baseline
```

```bash
pnpm check:typecheck-baseline
```

The lint, type and Clippy gates are no-regression baselines: existing debt may go down, but a pull request must not add new findings.

## Pull requests

- Keep each pull request focused on one change.
- Describe what changed and how you tested it.
- Use [Conventional Commits](https://www.conventionalcommits.org/) for commit messages (`feat:`, `fix:`, `docs:`, `refactor:` ...). Release notes are generated from them.

## Contributor License Agreement

Before your first pull request can be merged, you need to agree to the [Contributor License Agreement](CLA.md). You keep the copyright to your contribution; the agreement gives the maintainer the right to distribute it under the project's licenses, including offering VortiDeck under other license terms in the future. To agree, add this line to your pull request description:

> I have read the CLA in CLA.md and agree to its terms for this and future contributions.

## Licensing of contributions

- Changes outside `packages/` and `examples/` are distributed under GPL-3.0-or-later.
- Changes to `packages/plugin-sdk` and `examples/` are distributed under the MIT License.
