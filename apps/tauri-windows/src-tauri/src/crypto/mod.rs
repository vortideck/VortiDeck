use base64::{engine::general_purpose::STANDARD as Base64, Engine as _};
use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    ChaCha20Poly1305, Nonce,
};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use x25519_dalek::{EphemeralSecret, PublicKey, SharedSecret};

type HmacSha256 = Hmac<Sha256>;

/// E2EE keys for a paired device
pub struct E2EEKeys {
    pub desktop_private: Option<EphemeralSecret>,
    pub desktop_public: PublicKey,
    pub shared_secret: Option<[u8; 32]>,
}

impl E2EEKeys {
    /// Generate new desktop key pair for pairing
    pub fn generate() -> Self {
        let desktop_private = EphemeralSecret::random_from_rng(OsRng);
        let desktop_public = PublicKey::from(&desktop_private);

        Self {
            desktop_private: Some(desktop_private),
            desktop_public,
            shared_secret: None,
        }
    }

    /// Derive shared secret from mobile's public key
    pub fn derive_shared_secret(&mut self, mobile_public_key: &PublicKey) -> Result<(), String> {
        let desktop_private = self
            .desktop_private
            .take()
            .ok_or("Desktop private key already used")?;

        let shared_secret = desktop_private.diffie_hellman(mobile_public_key);
        self.shared_secret = Some(*shared_secret.as_bytes());

        Ok(())
    }

    /// Encrypt data (for token transmission)
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, String> {
        let shared_secret = self
            .shared_secret
            .as_ref()
            .ok_or("Shared secret not derived")?;

        encrypt_with_secret(shared_secret, plaintext)
    }

    /// Decrypt data (for receiving encrypted messages)
    pub fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>, String> {
        let shared_secret = self
            .shared_secret
            .as_ref()
            .ok_or("Shared secret not derived")?;

        decrypt_with_secret(shared_secret, ciphertext)
    }
}

/// Helper: Create HMAC signature (for messages)
pub fn create_hmac_signature(secret: &[u8], message: &str) -> String {
    let mut mac =
        <HmacSha256 as Mac>::new_from_slice(secret).expect("HMAC can take key of any size");

    mac.update(message.as_bytes());
    Base64.encode(mac.finalize().into_bytes())
}

/// Helper: Verify HMAC signature
pub fn verify_hmac_signature(secret: &[u8], message: &str, signature_b64: &str) -> bool {
    let signature = match Base64.decode(signature_b64) {
        Ok(sig) => sig,
        Err(_) => return false,
    };

    let mut mac =
        <HmacSha256 as Mac>::new_from_slice(secret).expect("HMAC can take key of any size");

    mac.update(message.as_bytes());

    mac.verify_slice(&signature).is_ok()
}

/// Sign a canonical JSON object for direct desktop-to-mobile delivery.
pub fn sign_json_message(
    shared_secret: &[u8; 32],
    mut message: serde_json::Value,
) -> Result<serde_json::Value, String> {
    message["nonce"] = serde_json::json!(uuid::Uuid::new_v4().to_string());
    let canonical = serde_json_canonicalizer::to_string(&message)
        .map_err(|e| format!("Canonical JSON error: {}", e))?;
    message["signature"] = serde_json::json!(create_hmac_signature(shared_secret, &canonical));
    Ok(message)
}

/// Helper: Encrypt data with shared secret (standalone function)
pub fn encrypt_with_secret(shared_secret: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, String> {
    let cipher = ChaCha20Poly1305::new(shared_secret.into());

    let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|e| format!("Encryption failed: {}", e))?;

    // Prepend nonce to ciphertext
    let mut result = nonce.to_vec();
    result.extend_from_slice(&ciphertext);
    Ok(result)
}

/// Helper: Decrypt data with shared secret (standalone function)
pub fn decrypt_with_secret(shared_secret: &[u8; 32], ciphertext: &[u8]) -> Result<Vec<u8>, String> {
    if ciphertext.len() < 12 {
        return Err("Ciphertext too short".to_string());
    }

    let cipher = ChaCha20Poly1305::new(shared_secret.into());

    let (nonce_bytes, encrypted_data) = ciphertext.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    cipher
        .decrypt(nonce, encrypted_data)
        .map_err(|e| format!("Decryption failed: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_exchange() {
        // Desktop generates keys
        let mut desktop_keys = E2EEKeys::generate();
        let desktop_public = desktop_keys.desktop_public;

        // Mobile generates keys
        let mobile_private = EphemeralSecret::random_from_rng(OsRng);
        let mobile_public = PublicKey::from(&mobile_private);

        // Both derive shared secret
        desktop_keys.derive_shared_secret(&mobile_public).unwrap();
        let mobile_shared_secret = mobile_private.diffie_hellman(&desktop_public);

        // Secrets should match
        assert_eq!(
            desktop_keys.shared_secret.unwrap(),
            *mobile_shared_secret.as_bytes()
        );
    }

    #[test]
    fn test_encryption() {
        let mut keys = E2EEKeys::generate();
        let mobile_private = EphemeralSecret::random_from_rng(OsRng);
        let mobile_public = PublicKey::from(&mobile_private);

        keys.derive_shared_secret(&mobile_public).unwrap();

        let plaintext = b"Hello, World!";
        let encrypted = keys.encrypt(plaintext).unwrap();
        let decrypted = keys.decrypt(&encrypted).unwrap();

        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_hmac_signing() {
        let secret = b"test_secret";
        let message = "test message";

        let signature = create_hmac_signature(secret, message);
        assert!(verify_hmac_signature(secret, message, &signature));

        // Wrong message should fail
        assert!(!verify_hmac_signature(secret, "wrong message", &signature));

        // Wrong secret should fail
        assert!(!verify_hmac_signature(b"wrong_secret", message, &signature));
    }

    #[test]
    fn benchmark_hmac_performance() {
        use std::time::Instant;

        let secret = b"test_secret_key_32_bytes_long!!!";
        let message = r#"{"action":"button_press","buttonId":"abc-123","timestamp":1234567890}"#;

        // Benchmark signing
        let iterations = 10000;
        let start = Instant::now();
        for _ in 0..iterations {
            let _signature = create_hmac_signature(secret, message);
        }
        let sign_duration = start.elapsed();
        let sign_avg_micros = sign_duration.as_micros() / iterations;

        // Benchmark verification
        let signature = create_hmac_signature(secret, message);
        let start = Instant::now();
        for _ in 0..iterations {
            assert!(verify_hmac_signature(secret, message, &signature));
        }
        let verify_duration = start.elapsed();
        let verify_avg_micros = verify_duration.as_micros() / iterations;

        println!(
            "\n🔐 E2EE Performance Benchmarks ({} iterations):",
            iterations
        );
        println!(
            "├─ HMAC Sign:   {} total, {}.{} μs avg per operation",
            format_duration(sign_duration),
            sign_avg_micros,
            (sign_duration.as_nanos() / iterations % 1000) / 100
        );
        println!(
            "├─ HMAC Verify: {} total, {}.{} μs avg per operation",
            format_duration(verify_duration),
            verify_avg_micros,
            (verify_duration.as_nanos() / iterations % 1000) / 100
        );
        println!(
            "└─ Total overhead per message: ~{}.{} μs (sign + verify)\n",
            sign_avg_micros + verify_avg_micros,
            ((sign_duration.as_nanos() + verify_duration.as_nanos()) / iterations % 1000) / 100
        );

        // Assert performance requirements (should be under 100μs per operation)
        assert!(
            sign_avg_micros < 100,
            "HMAC signing too slow: {} μs",
            sign_avg_micros
        );
        assert!(
            verify_avg_micros < 100,
            "HMAC verification too slow: {} μs",
            verify_avg_micros
        );
    }

    #[test]
    fn benchmark_key_exchange() {
        use std::time::Instant;

        println!("\n🔐 Key Exchange Performance:");

        // Benchmark desktop key generation
        let start = Instant::now();
        let mut desktop_keys = E2EEKeys::generate();
        let keygen_duration = start.elapsed();
        println!(
            "├─ Desktop key generation: {}",
            format_duration(keygen_duration)
        );

        // Benchmark mobile key generation
        let start = Instant::now();
        let mobile_private = EphemeralSecret::random_from_rng(OsRng);
        let mobile_public = PublicKey::from(&mobile_private);
        let mobile_keygen_duration = start.elapsed();
        println!(
            "├─ Mobile key generation:  {}",
            format_duration(mobile_keygen_duration)
        );

        // Benchmark shared secret derivation
        let start = Instant::now();
        desktop_keys.derive_shared_secret(&mobile_public).unwrap();
        let derive_duration = start.elapsed();
        println!(
            "├─ Shared secret derivation: {}",
            format_duration(derive_duration)
        );

        let total_handshake = keygen_duration + mobile_keygen_duration + derive_duration;
        println!(
            "└─ Total handshake time: {}\n",
            format_duration(total_handshake)
        );

        // Handshake should complete in under 1ms
        assert!(total_handshake.as_millis() < 10, "Key exchange too slow");
    }

    #[test]
    fn benchmark_encryption() {
        use std::time::Instant;

        let mut keys = E2EEKeys::generate();
        let mobile_private = EphemeralSecret::random_from_rng(OsRng);
        let mobile_public = PublicKey::from(&mobile_private);
        keys.derive_shared_secret(&mobile_public).unwrap();

        // Test different message sizes
        let messages = vec![
            ("Small (100B)", vec![b'x'; 100]),
            ("Medium (1KB)", vec![b'x'; 1024]),
            ("Large (10KB)", vec![b'x'; 10240]),
        ];

        println!("\n🔐 Encryption Performance:");
        for (label, plaintext) in messages {
            let iterations = 1000;

            // Benchmark encryption
            let start = Instant::now();
            let mut encrypted = vec![];
            for _ in 0..iterations {
                encrypted = keys.encrypt(&plaintext).unwrap();
            }
            let encrypt_duration = start.elapsed();
            let encrypt_avg_micros = encrypt_duration.as_micros() / iterations;

            // Benchmark decryption
            let start = Instant::now();
            for _ in 0..iterations {
                let _decrypted = keys.decrypt(&encrypted).unwrap();
            }
            let decrypt_duration = start.elapsed();
            let decrypt_avg_micros = decrypt_duration.as_micros() / iterations;

            println!(
                "├─ {} - Encrypt: {}.{} μs, Decrypt: {}.{} μs",
                label,
                encrypt_avg_micros,
                (encrypt_duration.as_nanos() / iterations % 1000) / 100,
                decrypt_avg_micros,
                (decrypt_duration.as_nanos() / iterations % 1000) / 100
            );
        }
        println!("└─ Note: Full encryption is optional - HMAC signing is the primary security\n");
    }

    fn format_duration(duration: std::time::Duration) -> String {
        if duration.as_millis() > 0 {
            format!(
                "{}.{:02}ms",
                duration.as_millis(),
                (duration.as_micros() % 1000) / 10
            )
        } else {
            format!(
                "{}.{} μs",
                duration.as_micros(),
                (duration.as_nanos() % 1000) / 100
            )
        }
    }
}
