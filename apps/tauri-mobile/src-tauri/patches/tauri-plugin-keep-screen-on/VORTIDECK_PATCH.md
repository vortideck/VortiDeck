# Local patch of tauri-plugin-keep-screen-on 0.1.4

Upstream: https://gitlab.com/cristofa/tauri-plugin-keep-screen-on (MIT OR Apache-2.0).

Copied unchanged from the crates.io 0.1.4 release except for one line:
`android/build.gradle.kts` no longer sets `defaultConfig.targetSdk`.
Android Gradle Plugin 9 (used by Tauri 2.12's Android template) removed
`targetSdk` from library modules, so the original file fails to compile.
The value had no effect for a library.

Used through `[patch.crates-io]` in `../../Cargo.toml`. Remove this folder and
the patch entry once an upstream release builds with AGP 9.

License: MIT OR Apache-2.0 (upstream). The MIT text is in LICENSE-MIT.
