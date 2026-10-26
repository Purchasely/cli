export const EXIT = {
  OK: 0,
  GENERIC: 1,
  USAGE: 2,
  AUTH: 3,
  FORBIDDEN: 4,
  VALIDATION: 5,
  NOT_FOUND: 6,
  RATE_LIMITED: 7,
  CONFIRMATION: 8,
  PARTIAL: 9,
  REFUSED: 10,
} as const;

export class CliError extends Error {
  constructor(
    public exit: number,
    public code: string,
    public messages: string[],
    public extra: Record<string, unknown> = {},
    public status?: number,
  ) {
    super(messages.join("; "));
  }
  toJSON() {
    const { confirm_token, ...rest } = this.extra;
    const error: Record<string, unknown> = { code: this.code, messages: this.messages };
    if (this.status !== undefined) error.status = this.status;
    return { error: { ...error, ...rest }, ...(confirm_token ? { confirm_token } : {}) };
  }
}

export const usage = (msg: string) => new CliError(EXIT.USAGE, "USAGE", [msg]);

// Error code first, HTTP status as fallback.
export function exitFor(code: string | undefined, status: number): number {
  switch (code) {
    case "UNAUTHORIZED": return EXIT.AUTH;
    case "FORBIDDEN":
    case "FEATURE_NOT_ENABLED": return EXIT.FORBIDDEN;
    case "VALIDATION_FAILED": return EXIT.VALIDATION;
    case "NOT_FOUND": return EXIT.NOT_FOUND;
    case "RATE_LIMITED": return EXIT.RATE_LIMITED;
    case "CONFIRMATION_REQUIRED": return EXIT.CONFIRMATION;
  }
  if (status === 401) return EXIT.AUTH;
  if (status === 403) return EXIT.FORBIDDEN;
  if (status === 404) return EXIT.NOT_FOUND;
  if (status === 422) return EXIT.VALIDATION;
  if (status === 429) return EXIT.RATE_LIMITED;
  return EXIT.GENERIC;
}
