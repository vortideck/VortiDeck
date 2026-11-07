//! First-party package integrity verification — the gate that makes "builtins as
//! editable files" safe.
//!
//! Trust is conferred by VERIFICATION, never by folder location. A first-party
//! builtin package is pinned here (package_id → sha256 of its widget content),
//! and the pin is compiled into the binary, which is the trust root (the user
//! installed the app; anything bundled in it is trusted a priori). On load the
//! package content is hashed and compared: a match earns `builtin` trust (the
//! in-process fast path + full APIs); a mismatch — i.e. someone edited the file
//! — downgrades it to `community` (sandboxed). So editing a builtin destroys its
//! trust; you cannot escalate to the fast path by editing a file.
//!
//! The table starts empty and is populated as builtins are migrated to packages
//! (see the unified-architecture plan, Phase 5). Until then every folder-loaded
//! package is `community` — which is correct, because the plugins folder is
//! user-writable (`%APPDATA%\VortiDeck\plugins`), not a first-party source.
//!
//! Ed25519 signature verification (`plugins::signing`) is the robust companion
//! for marketplace-distributed "verified creator" packages (Phase 2/6); this
//! hash-pin covers first-party builtins with no key management.

use sha2::{Digest, Sha256};

/// First-party builtin pins: `(package_id, sha256(widget content))`. Compiled in.
/// Add entries here as builtins are packaged (Phase 5). The hex digest is the
/// sha256 of the package's widget contribution `content` string, lowercase.
const BUILTIN_HASHES: &[(&str, &str)] = &[
    // ("builtin-counter", "<sha256 hex of counter widget content>"),
];

/// True if `id` is a known first-party builtin package (has a pinned hash).
pub fn is_known_builtin(id: &str) -> bool {
    BUILTIN_HASHES.iter().any(|(pid, _)| *pid == id)
}

/// Verify a package's widget content against its pinned builtin hash.
///
/// Returns true ONLY for an unmodified first-party package — the gate for
/// `builtin` trust. `content` is the widget contribution's HTML/CSS/JS string.
pub fn verify_builtin_hash(id: &str, content: &str) -> bool {
    let expected = match BUILTIN_HASHES.iter().find(|(pid, _)| *pid == id) {
        Some((_, h)) => *h,
        None => return false,
    };
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let actual: String = hasher
        .finalize()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect();
    actual == expected
}
