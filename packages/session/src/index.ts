import type { components } from "@vikingthedev/api-types";

export type SessionUser = components["schemas"]["SessionUser"];

const SESSION_COOKIE_NAME = "__Secure-vt_sid";
const SESSION_PATH = "/session";
// A slow API degrades the page to anonymous instead of stalling it (frontend.md).
const SESSION_LOOKUP_TIMEOUT_MS = 50;
const UNAUTHORIZED = 401;

export interface SessionLookup {
  apiInternalUrl: string;
  cookieHeader: string | null;
}

/** Keeps only the session cookies, repeats included, so the API still sees several distinct tokens as anonymous. */
export function selectSessionCookies(
  cookieHeader: string | null,
): string | null {
  const sessionCookies = (cookieHeader ?? "")
    .split(";")
    .map((cookie) => cookie.trim())
    .filter((cookie) => cookie.startsWith(`${SESSION_COOKIE_NAME}=`));
  return sessionCookies.length === 0 ? null : sessionCookies.join("; ");
}

/** Asks the internal API for the session user; anonymous (`null`) on a missing session, an error or a timeout. */
export async function loadSessionUser({
  apiInternalUrl,
  cookieHeader,
}: SessionLookup): Promise<SessionUser | null> {
  const sessionCookies = selectSessionCookies(cookieHeader);
  if (sessionCookies === null) {
    return null;
  }
  try {
    const response = await fetch(new URL(SESSION_PATH, apiInternalUrl), {
      headers: { cookie: sessionCookies },
      signal: AbortSignal.timeout(SESSION_LOOKUP_TIMEOUT_MS),
    });
    if (!response.ok) {
      // An unread body keeps the pooled connection busy.
      await response.body?.cancel();
      logUnexpectedStatus(response.status);
      return null;
    }
    return (await response.json()) as SessionUser;
  } catch (error) {
    console.error("session lookup failed", { error: errorName(error) });
    return null;
  }
}

function logUnexpectedStatus(status: number): void {
  if (status !== UNAUTHORIZED) {
    console.error("session lookup failed", { status });
  }
}

// Only the name: an error's message could echo the request, which carries the session cookie.
function errorName(error: unknown): string {
  return error instanceof Error ? error.name : "unknown";
}
