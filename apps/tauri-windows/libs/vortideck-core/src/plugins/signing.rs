//! Ed25519 signature verification for plugin/widget packages — the robust
//! companion to hash-pinning (`integrity.rs`).
//!
//! A package signed by a trusted key (the first-party key bundled with the app,
//! or a marketplace verified-creator key) verifies here; a valid signature earns
//! `builtin`/`verified` trust — independent of file location. This is pure
//! VERIFICATION: the host never signs. Signing happens at packaging time (a
//! build/CLI tool, Phase 5/6), producing a `signature.json` (`bundle::PluginSignature`).
//!
//! Signing convention: the signature is over the package's sha256 digest; on a
//! `PluginSignature`, `package_hash` is that digest as lowercase hex, and
//! `signature`/`author_pubkey` are base64 (standard). `verify_plugin_signature`
//! decodes + checks them against a caller-supplied trusted key.
//!
//! See the unified-architecture plan (Phase 2).

use base64::Engine;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};

/// Verify an Ed25519 signature over `message` with a 32-byte `public_key`
/// (signature is 64 bytes). True only for a valid signature; never panics
/// (malformed inputs return false).
pub fn verify_signature(message: &[u8], signature: &[u8], public_key: &[u8]) -> bool {
    let pk: &[u8; 32] = match public_key.try_into() {
        Ok(pk) => pk,
        Err(_) => return false, // public key must be exactly 32 bytes
    };
    let Ok(vk) = VerifyingKey::from_bytes(pk) else {
        return false;
    };
    let Ok(sig) = Signature::from_slice(signature) else {
        return false;
    }; // must be 64 bytes
    vk.verify(message, &sig).is_ok()
}

/// Verify a `PluginSignature` against a caller-supplied trusted public key
/// (raw 32 bytes). The caller resolves which key to trust — the bundled
/// first-party key, or a marketplace verified-creator key indexed by
/// `sig.author_id`. Returns false on any malformed field.
pub fn verify_plugin_signature(
    sig: &super::bundle::PluginSignature,
    trusted_pubkey: &[u8],
) -> bool {
    // The signature is over the 32-byte sha256 digest; package_hash is its hex form.
    let digest = match hex::decode(sig.package_hash.trim()) {
        Ok(d) if d.len() == 32 => d,
        _ => return false,
    };
    let signature = match base64::engine::general_purpose::STANDARD.decode(sig.signature.as_bytes())
    {
        Ok(s) if s.len() == 64 => s,
        _ => return false,
    };
    verify_signature(&digest, &signature, trusted_pubkey)
}
