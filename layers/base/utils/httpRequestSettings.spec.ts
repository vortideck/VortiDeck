import { describe, expect, it } from 'vitest';
import { acceptsHttpStatus, isProtectedHttpHeader, parseAcceptedHttpStatuses } from './httpRequestSettings';

describe('HTTP request response policy', () => {
  it('defaults to successful responses and supports intentional error responses', () => {
    expect(acceptsHttpStatus(201, parseAcceptedHttpStatuses('2xx'))).toBe(true);
    expect(acceptsHttpStatus(500, parseAcceptedHttpStatuses('2xx'))).toBe(false);
    const policy = parseAcceptedHttpStatuses('2xx, 404, 404');
    expect(acceptsHttpStatus(404, policy)).toBe(true);
    expect(acceptsHttpStatus(500, policy)).toBe(false);
    expect(policy?.filter(code => code === 404)).toHaveLength(1);
    expect(acceptsHttpStatus(503, parseAcceptedHttpStatuses('*'))).toBe(true);
  });

  it('rejects incomplete or invalid policies rather than falling back to success', () => {
    for (const policy of ['', '200,', '200 404', '600', '99', 'everything']) {
      expect(() => parseAcceptedHttpStatuses(policy)).toThrow();
    }
  });

  it('recognizes case-insensitive credential headers and existing opaque references', () => {
    expect(isProtectedHttpHeader(' AUTHORIZATION ', 'new-token')).toBe(true);
    expect(isProtectedHttpHeader('Custom', 'vortideck-credential://identity')).toBe(true);
    expect(isProtectedHttpHeader('Content-Type', 'application/json')).toBe(false);
  });
});
