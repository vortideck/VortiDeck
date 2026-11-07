import { describe, expect, it } from "vitest";
import {
  createRelayPayloadEnvelope,
  decrypt,
  encrypt,
  signMessage,
  verifyRelayPayloadEnvelope,
  verifyMessage,
} from "./crypto";

describe("mobile E2EE transport", () => {
  const secret = new Uint8Array(32).fill(17);

  it("encrypts and authenticates sensitive pairing payloads", () => {
    const plaintext = JSON.stringify({ token: "secret-token", device: { type: "mobile" } });
    const ciphertext = encrypt(secret, plaintext);
    expect(ciphertext).not.toContain("secret-token");
    expect(decrypt(secret, ciphertext)).toBe(plaintext);
    expect(() => decrypt(new Uint8Array(32).fill(18), ciphertext)).toThrow();
  });

  it("signs canonical direct messages and detects tampering", () => {
    const signed = signMessage(secret, {
      action: "deck_widget_intent",
      payload: { z: 1, a: 2 },
    });
    expect(signed.protocol_version).toBe(2);
    expect(typeof signed.message_id).toBe("string");
    expect(signed.expires_at - signed.sent_at).toBe(30_000);
    expect(verifyMessage(secret, signed)).toBe(true);
    expect(verifyMessage(secret, { ...signed, action: "command" })).toBe(false);
    expect(verifyMessage(secret, { ...signed, signature: undefined })).toBe(false);
  });

  it("accepts one valid relay envelope and rejects replay, tampering, and unsigned data", () => {
    const payload = {
      action: "deck_widget_state",
      patch: { instanceId: "widget:1", revision: 2 },
    };
    const envelope = createRelayPayloadEnvelope(secret, payload);
    const usedNonces = new Set<string>();

    expect(verifyRelayPayloadEnvelope(secret, envelope, usedNonces)).toBe(true);
    expect(verifyRelayPayloadEnvelope(secret, envelope, usedNonces)).toBe(false);
    expect(verifyRelayPayloadEnvelope(secret, {
      ...envelope,
      nonce: "15c14ed5-8f2f-4e23-b5ed-4adca8178010",
      payload: { ...payload, action: "deck_widget_intent" },
    }, usedNonces)).toBe(false);
    expect(verifyRelayPayloadEnvelope(secret, { payload }, usedNonces)).toBe(false);
  });
});
