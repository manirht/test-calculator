/** Wire types for the `calc serve` JSON-lines protocol. */

export interface EvalOptions {
  places?: number;
  mode?: string;
}

export interface EvalResult {
  /** Decimal text, rounded per the request's places and mode. */
  value: string;
  /** Exact value as `n` or `n/d`. */
  exact: string;
}

export interface OkReply {
  id: number | null;
  ok: true;
  value?: string;
  exact?: string;
  functions?: string[];
}

export interface ErrReply {
  id: number | null;
  ok: false;
  error: { code: string; message: string; pos?: number };
}

export type Reply = OkReply | ErrReply;

export function isReply(value: unknown): value is Reply {
  if (typeof value !== "object" || value === null) return false;
  const reply = value as Record<string, unknown>;
  if (reply.ok === true) return true;
  if (reply.ok === false) {
    const error = reply.error as Record<string, unknown> | undefined;
    return typeof error?.code === "string" && typeof error?.message === "string";
  }
  return false;
}
