import { describe, expect, it } from "vitest";
import { shouldVerifySignature } from "./connectionHelpers";

describe("direct E2EE verification policy", () => {
  const secret = new Uint8Array(32).fill(1);

  it("requires verification even when an attacker omits signature fields", () => {
    expect(shouldVerifySignature(secret, { action: "deck_widget_snapshot" })).toBe(true);
    expect(shouldVerifySignature(secret, { action: "server_info" })).toBe(true);
  });

  it("exempts only the AEAD-authenticated initial token bootstrap", () => {
    expect(shouldVerifySignature(secret, {
      action: "new_token",
      encrypted_payload: "authenticated-ciphertext",
    })).toBe(false);
    expect(shouldVerifySignature(secret, { action: "new_token" })).toBe(true);
    expect(shouldVerifySignature(null, { action: "new_token" })).toBe(false);
  });

  it("does not require a duplicate inner signature after relay verification", () => {
    expect(shouldVerifySignature(secret, { action: "server_info" }, true)).toBe(false);
  });
});
