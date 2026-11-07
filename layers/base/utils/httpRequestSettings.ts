/** Default success is 2xx. Explicit policies can include codes and families. */
export function parseAcceptedHttpStatuses(text: string): number[] | null {
  const normalized = text.trim().toLowerCase();
  if (normalized === '2xx') return null;
  if (normalized === '*') return Array.from({ length: 500 }, (_, index) => index + 100);
  const codes = new Set<number>();
  for (const token of normalized.split(',').map(part => part.trim())) {
    if (/^[1-5]xx$/.test(token)) {
      const start = Number(token[0]) * 100;
      for (let code = start; code < start + 100; code++) codes.add(code);
    } else if (/^[1-5]\d{2}$/.test(token)) {
      codes.add(Number(token));
    } else {
      throw new Error('Use response codes such as 200, 404, families such as 2xx, or * for all statuses.');
    }
  }
  return [...codes].sort((left, right) => left - right);
}

export function acceptsHttpStatus(status: number, codes: number[] | null): boolean {
  return codes ? codes.includes(status) : status >= 200 && status < 300;
}

export function isProtectedHttpHeader(name: string, value: string): boolean {
  return value.startsWith('vortideck-credential://') ||
    /^(authorization|proxy-authorization|cookie|x-api-key|api-key|x-auth-token)$/i.test(name.trim());
}
