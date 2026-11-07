import { describe, expect, it, vi } from "vitest";
import { reportAuthFailure, safeAuthDiagnostic } from "@base/utils/authDiagnostics";

describe("auth diagnostics", () => {
  it("preserves useful status/network evidence without logging credential values", () => {
    const secret = "seeded-secret-value";
    const error = Object.assign(
      new Error(`network request failed: Authorization: Bearer ${secret}`),
      {
        response: {
          status: 503,
          headers: { authorization: `Bearer ${secret}` },
          data: { refresh_token: secret },
        },
      },
    );
    const logger = { error: vi.fn() };

    reportAuthFailure("synthetic refresh", error, logger);

    expect(safeAuthDiagnostic(error)).toEqual({
      status: 503,
      networkError: true,
      tokenNotFound: false,
    });
    expect(JSON.stringify(logger.error.mock.calls)).not.toContain(secret);
    expect(logger.error).toHaveBeenCalledWith(
      "[Auth] synthetic refresh failed",
      { status: 503, networkError: true, tokenNotFound: false },
    );
  });

  it("does not serialize arbitrary thrown objects", () => {
    const secret = "another-seeded-secret";
    const logger = { error: vi.fn() };

    reportAuthFailure("verification", {
      status: 401,
      token: secret,
      toString: () => `authorization=${secret}`,
    }, logger);

    expect(JSON.stringify(logger.error.mock.calls)).not.toContain(secret);
    expect(logger.error.mock.calls[0]?.[1]).toEqual({
      status: 401,
      networkError: false,
      tokenNotFound: false,
    });
  });
});
