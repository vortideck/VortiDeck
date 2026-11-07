# OBS Companion mDNS Discovery Architecture and Cleanup Plan

## Status

The production discovery fix and follow-up cleanup are implemented and released in OBS Companion `v1.0.6` from source commit `fa7a268efd1bc5e7bd6a9ab59b98216797253f08`.

The end-to-end advertiser uses only the low-level packet API, unused wrapper sources are absent from every VortiDeck-owned CMake target, Refresh preserves the active service's connected status, and non-connected discoveries are shown as available rather than failed.

VortiDeck now embeds the verified `v1.0.6` artifact with SHA-256:

`58B463A3463F4CED5FB779715A9A6574D03E1DCA4AB967BED6BC378510578B34`

GitHub Actions built and published the release from a clean checkout. The desktop staging script independently verified the release assets, SHA-256, compatibility manifest, source provenance, architecture, and unsigned development signature state before embedding the DLL.

## What changed

The former OBS implementation treated human-readable `mdns_cpp` diagnostic logs as discovery protocol data. It searched text messages for PTR, SRV, and address information. That design was fragile because log formatting is not a stable API.

OBS Companion `v1.0.6` now:

- reads DNS-SD records directly from received multicast DNS packets;
- decodes PTR, SRV, A, AAAA, and TXT records into typed structures;
- assembles records independently of packet and record ordering;
- deduplicates resolved services;
- supports IPv4 and bracketed IPv6 WebSocket URLs;
- streams completed services immediately into the automatic connection path;
- reports discovery failures in the OBS log;
- permits an active scan to be cancelled within a bounded interval;
- performs and awaits a real scan when the user presses Refresh;
- adds no hard-coded loopback endpoint or discovery fallback.

The end-to-end test advertises a real `_vortideck._tcp.local.` service and resolves it through multicast DNS packets. A live probe also resolved the running VortiDeck instance as:

`vortideck_1._vortideck._tcp.local. -> ws://192.168.1.203:9001/ws`

## Current dependency boundary

There are two distinct layers:

1. **VortiDeck-owned discovery logic**
   - `structured_mdns_query.cpp` decodes raw DNS records into typed records.
   - `mdns_discovery.cpp` assembles those records into VortiDeck service endpoints.
   - The OBS worker controls automatic scans, Refresh, logging, cancellation, and reconnection behavior.

2. **Pinned third-party packet implementation**
   - The `mdns_cpp` submodule contains the low-level single-header `mdns.h` packet and socket implementation.
   - The high-level `mdns_cpp::mDNS` C++ wrapper is no longer used by production discovery.
   - The high-level wrapper is not compiled or used by production or test targets.
   - The end-to-end test owns a small low-level advertiser with deterministic readiness and shutdown.

This means VortiDeck owns the behavior and stable typed interface, while a pinned dependency still supplies low-level DNS packet primitives.

## Do we need to remove the submodule?

Not immediately, and we should not rewrite the complete mDNS protocol without a concrete reason.

The submodule itself is not broken. Its high-level C++ wrapper simply does not provide the structured query API VortiDeck needs. Replacing log parsing with an application-owned adapter was the correct boundary.

The recommended short-term design is:

- retain the pinned submodule revision;
- use only its low-level `mdns.h` implementation in production;
- keep all VortiDeck-specific discovery and connection policy in VortiDeck-owned code;
- do not add private modifications to the submodule;
- do not parse diagnostic logger output.

This is already safe and reproducible because CI builds from the clean pinned revision.

## Recommended final cleanup

The next cleanup should remove dependency on the **high-level `mdns_cpp` wrapper**, not necessarily remove every third-party mDNS primitive.

### Phase 1: Replace the test advertiser

Create a small test-only advertiser based on the same low-level `mdns.h` packet API. It should:

- bind the appropriate multicast sockets;
- advertise a uniquely named `_vortideck._tcp.local.` instance;
- answer PTR and SRV queries and include A/AAAA data;
- use bounded polling and deterministic shutdown;
- expose readiness to the test without arbitrary sleeps;
- report initialization failures as test failures rather than terminating a worker thread.

After this change, no production or test code should instantiate `mdns_cpp::mDNS` or use its global logger.

### Phase 2: Stop compiling unused wrapper sources

Remove these sources from the OBS plugin and test CMake targets when no references remain:

- `other/mdns_cpp/src/mdns.cpp`
- `other/mdns_cpp/src/logger.cpp`
- `other/mdns_cpp/src/utils.cpp`

Keep only the include path required for the pinned low-level `mdns.h` implementation.

Acceptance checks:

- `rg "mdns_cpp::mDNS|mdns_cpp::Logger|mdns_cpp/mdns.hpp"` finds no VortiDeck-owned source references;
- the OBS plugin builds on a clean checkout;
- the structured policy test passes;
- the self-contained mDNS end-to-end test passes;
- the live VortiDeck discovery probe passes;
- GitHub release CI passes from a clean submodule revision.

### Phase 3: Decide how to package the low-level dependency

Once the wrapper has been removed, choose one of these intentionally:

#### Option A: Keep the pinned submodule

This is the lowest-risk option.

- Continue pinning `mdns_cpp` to an immutable commit.
- Include only its low-level `mdns.h` from VortiDeck code.
- Record its upstream repository, commit, and license.
- Use automated dependency review for future updates.

Advantages:

- minimal code churn;
- upstream history remains visible;
- updates can be reviewed as explicit submodule pointer changes;
- no copied third-party implementation is disguised as application code.

Disadvantages:

- the checkout still contains wrapper code that VortiDeck does not compile;
- submodule initialization remains part of contributor and CI setup.

#### Option B: Vendor only the required low-level library

This can simplify checkout and packaging after the dependency boundary is proven stable.

- Copy the exact upstream `mdns.h` release into a clearly named third-party directory.
- Preserve its complete copyright and license notice.
- add a provenance file containing upstream URL, commit/tag, and checksum;
- do not modify the vendored file for VortiDeck-specific behavior;
- keep the typed adapter outside the third-party directory;
- add an explicit update procedure and dependency/security review.

Advantages:

- no Git submodule required;
- the repository contains only the dependency code it actually uses;
- clean checkout and CI setup become simpler.

Disadvantages:

- VortiDeck must track upstream releases and security fixes itself;
- careless vendoring can obscure provenance and licensing;
- copied code may drift if local patches are allowed.

#### Option C: Adopt another maintained DNS-SD library

Only select this after a documented evaluation of:

- Windows and OBS compatibility;
- IPv4 and IPv6 behavior;
- cancellation and bounded timeouts;
- structured record callbacks;
- thread-safety and lifecycle semantics;
- binary size and external runtime dependencies;
- licensing and maintenance activity;
- deterministic automated testing.

This option has the highest migration risk and is unnecessary while the current low-level implementation remains adequate.

## Recommended decision

Use **Option A until the next deliberate dependency migration**, then adopt **Option B as the long-term layout**.

In practical terms:

- VortiDeck already owns the important structured discovery architecture.
- Do not build a new mDNS protocol stack from scratch.
- The unused wrapper has been removed from build targets after replacing the test advertiser.
- Vendoring the one required public-domain `mdns.h` file is now operationally simpler than retaining an entire recursive submodule: it removes submodule initialization and dirty-worktree failure modes while keeping VortiDeck-specific behavior outside the third-party file.
- The vendored copy must include an upstream URL, exact commit, SHA-256, license/provenance notice, and an automated update/check procedure. Until that migration is performed and verified in clean CI, the pinned submodule remains the safer reproducible input.

## Local dirty submodule state

The former local changes to `other/mdns_cpp/include/mdns_cpp/mdns.hpp` and `other/mdns_cpp/src/mdns.cpp` were backed up before cleanup to:

a local `mdns_cpp` patch file (kept outside this repository)

The backup SHA-256 is:

`117E84FA647A568905F2F152FE6D0B6946FD2E4C50144B369A3B95A2FF2E32A5`

After explicit approval, those two files were restored to the pinned submodule commit `05b181ca2b3920b787287a291fae326ecd0ef019`. The submodule is clean. The completed procedure was:

1. Export the local submodule diff to a backup patch outside the submodule.
2. Review whether any change is still intentional.
3. Confirm that clean CI already covers the required production behavior.
4. Restore the two reviewed files to the pinned commit after approval.
5. Re-run the plugin build, structured policy test, and end-to-end test.

No submodule modification should be committed merely to make local status appear clean.

## Remaining implementation checklist

- [x] Replace log-text parsing with structured DNS-SD records.
- [x] Support order-independent PTR/SRV/A/AAAA assembly.
- [x] Stream discovered services into automatic connection.
- [x] Make Refresh wake and await a real scan.
- [x] Report discovery failures in the OBS log.
- [x] Add structured-discovery policy protection.
- [x] Add real advertisement/discovery end-to-end coverage.
- [x] Validate discovery against a running VortiDeck instance.
- [x] Build and publish OBS Companion `v1.0.6` from clean CI.
- [x] Embed the verified `v1.0.6` artifact in VortiDeck.
- [x] Pass the Rust transactional companion-manager tests.
- [x] Replace the end-to-end test's `mdns_cpp::mDNS` advertiser with a test-owned low-level advertiser.
- [x] Remove unused wrapper source files from CMake build targets.
- [x] Back up and review the local dirty submodule worktree.
- [x] With explicit approval, restore the reviewed local submodule files to pinned commit `05b181c`.
- [x] Decide whether retaining the pinned submodule or vendoring only `mdns.h` is operationally simpler: vendor only `mdns.h` in the long-term layout, with pinned provenance and automated verification.

## Operational note

VortiDeck embeds OBS Companion `v1.0.6`, but an existing installed older copy remains unchanged until the user selects Update in the VortiDeck OBS Companion manager and restarts OBS. Installation and updates must continue through that manager so ownership receipts, architecture validation, staging, backup, verification, rollback, repair, and removal semantics remain intact.
