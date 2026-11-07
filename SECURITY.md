# Security Policy

## Reporting a vulnerability

Please do not report security problems in public issues.

Use GitHub's private vulnerability reporting on this repository (Security tab → "Report a vulnerability"), or email security@vortideck.com. Include what you found, how to reproduce it, and the version you tested.

You can expect an acknowledgement within a few days. Once a fix is released, the report can be disclosed and credited if you want.

## Supported versions

Security fixes go into the latest released desktop and mobile versions. Older versions are not patched.

## Scope notes

- VortiDeck runs a local server for the phone companion, OBS overlays and webhooks. Reports about exposure of that server to other devices or websites are in scope.
- Plugins are installed with a per-package permission grant. Native service plugins run as normal programs with your user rights; that is by design and is shown during installation.
