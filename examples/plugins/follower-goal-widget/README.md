# Follower Goal widget fixture

This package is the source-controlled community-widget example used to verify
VortiDeck's sandbox, consent, subscription filtering, responsive rendering and
saved-overlay reconciliation. It is intentionally a community package rather
than part of the embedded `builtin.vortideck-widgets` package.

- Package ID: `demo-follower-goal` (kept stable for existing test installs)
- Contribution ID: `follower-goal`
- Granted input: canonical `follow` events only
- Network and outbound actions: none
- Source of truth: `manifest.json`
- Installable artifact: `../follower-goal-widget.vdext`

After changing executable contribution content or capabilities, increment the
manifest version, rebuild the `.vdext` archive, and verify the package lifecycle
pauses old saved content until the current contribution is reconciled.
