export interface SafeAuthDiagnostic {
  status?: number;
  networkError: boolean;
  tokenNotFound: boolean;
}

const errorText = (error: unknown): string => {
  try {
    return String(error);
  } catch {
    return "";
  }
};

const responseStatus = (error: unknown): number | undefined => {
  if (!error || typeof error !== "object") return undefined;
  const record = error as { status?: unknown; response?: { status?: unknown } };
  const status = record.response?.status ?? record.status;
  return typeof status === "number" && Number.isFinite(status) ? status : undefined;
};

export const safeAuthDiagnostic = (error: unknown): SafeAuthDiagnostic => {
  const text = errorText(error).toLowerCase();
  return {
    status: responseStatus(error),
    networkError: text.includes("fetch")
      || text.includes("network")
      || text.includes("failed to send request")
      || text.includes("econnrefused")
      || text.includes("enotfound"),
    tokenNotFound: text.includes("token not found"),
  };
};

export const reportAuthFailure = (
  operation: string,
  error: unknown,
  logger: Pick<Console, "error"> & Partial<Pick<Console, "warn">> = console,
  level: "error" | "warn" = "error",
): void => {
  const write = level === "warn" ? (logger.warn ?? logger.error) : logger.error;
  write(`[Auth] ${operation} failed`, safeAuthDiagnostic(error));
};
