# VortiDeck desktop

The Windows-first Tauri host for VortiDeck's local control deck, workflow
runtime, plugin system, streaming integrations and overlay studio.

## Source map

- `nuxt.config.ts`, `app.vue`: desktop frontend host.
- `src-tauri`: Tauri process, commands, permissions and platform adapters.
- `../libs/vortideck-core`: shared runtime, persistence and domain services.
- `../../layers/desktop`: desktop shell and product pages.
- `../../layers/desktop-streaming`: streaming and overlay studio UI.
- `../../layers/flow`: flow editor and execution UI.
- `../../layers/base`: host-neutral components, stores and IPC contracts.

The current task order and acceptance criteria live in
`../../TODO.md`. Do not use `new_architecture` or archived
planning documents as implementation truth.

## Develop

From the repository root:

```powershell
pnpm install
pnpm start:desktop
```

The desktop Nuxt server uses `http://127.0.0.1:4450`. Development builds may
expose CDP on `http://127.0.0.1:9222`; production builds must not.

Useful checks:

```powershell
pnpm --filter vortideck-desktop test
pnpm typecheck:desktop
pnpm lint:desktop
pnpm generate:desktop
pnpm check:desktop:rust
```

See the repository
`Agent command map` for scoped
Rust, SDK, mobile and release commands and for the current known diagnostic
baseline.

## Build

```powershell
pnpm build:desktop
```

Use `pnpm build:desktop:inspect` for the incremental packaged-inspection path.
Release packaging and lifecycle gates are owned by the workflows under
`.github/workflows`; a local frontend generation is not an installer acceptance
test.

## Runtime configuration

- AI providers are configured in Settings and stored through the application
  configuration/persistence boundary.
- MCP server configuration is stored under Tauri's AppConfig directory. The app
  creates the canonical file on first use and migrates the former AppData path
  when necessary; there is no source-controlled runtime `config.toml`.
- OBS companion installation and repair are managed from Settings rather than
  by copying a DLL from this source tree.

## Plugin development

The public authoring contract is
[`../../packages/plugin-sdk/README.md`](../../packages/plugin-sdk/README.md).
Run `pnpm generate:plugin-sdk` only when Rust host functions change; normal Rust
builds verify generated artifacts without rewriting them.
