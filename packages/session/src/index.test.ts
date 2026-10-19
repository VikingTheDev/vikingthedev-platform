import assert from "node:assert/strict";
import {
  createServer,
  type IncomingMessage,
  type ServerResponse,
} from "node:http";
import type { AddressInfo } from "node:net";
import { after, before, describe, it } from "node:test";

import {
  loadSessionUser,
  selectSessionCookies,
  type SessionUser,
} from "./index.ts";

const ADA: SessionUser = {
  user: {
    id: "0199f0a0-0000-7000-8000-000000000000",
    display_name: "Ada",
    created_at: "2026-10-19T00:00:00Z",
  },
  role: "user",
  grants: [],
};
const VALID_COOKIE = "__Secure-vt_sid=valid";
const SLOW_COOKIE = "__Secure-vt_sid=slow";
const BROKEN_COOKIE = "__Secure-vt_sid=broken";
const SLOWER_THAN_TIMEOUT_MS = 200;

describe("selectSessionCookies", () => {
  it("keeps only session cookies, repeats included", () => {
    assert.equal(
      selectSessionCookies(
        "theme=dark; __Secure-vt_sid=a; x__Secure-vt_sid=b; __Secure-vt_sid=c",
      ),
      "__Secure-vt_sid=a; __Secure-vt_sid=c",
    );
  });

  it("returns null without a session cookie", () => {
    assert.equal(selectSessionCookies(null), null);
    assert.equal(selectSessionCookies("theme=dark"), null);
  });
});

describe("loadSessionUser", () => {
  const forwardedCookies: (string | undefined)[] = [];
  const server = createServer(
    (request: IncomingMessage, response: ServerResponse) => {
      forwardedCookies.push(request.headers.cookie);
      respondAsTheApi(request, response);
    },
  );
  let apiInternalUrl = "";

  before(async () => {
    await new Promise<void>((resolve) =>
      server.listen(0, "127.0.0.1", resolve),
    );
    apiInternalUrl = `http://127.0.0.1:${(server.address() as AddressInfo).port}`;
  });
  after(() => {
    server.closeAllConnections();
    server.close();
  });

  it("returns the session user and forwards only the session cookie", async () => {
    const user = await loadSessionUser({
      apiInternalUrl,
      cookieHeader: `theme=dark; ${VALID_COOKIE}`,
    });

    assert.deepEqual(user, ADA);
    assert.equal(forwardedCookies.at(-1), VALID_COOKIE);
  });

  it("is anonymous without asking the API when there's no session cookie", async () => {
    const requestsBefore = forwardedCookies.length;

    assert.equal(
      await loadSessionUser({ apiInternalUrl, cookieHeader: "theme=dark" }),
      null,
    );
    assert.equal(forwardedCookies.length, requestsBefore);
  });

  it("is anonymous when the API rejects the session", async () => {
    const cookieHeader = "__Secure-vt_sid=unknown";

    assert.equal(await loadSessionUser({ apiInternalUrl, cookieHeader }), null);
  });

  it("is anonymous when the API fails, times out or is unreachable", async () => {
    assert.equal(
      await loadSessionUser({ apiInternalUrl, cookieHeader: BROKEN_COOKIE }),
      null,
    );
    assert.equal(
      await loadSessionUser({ apiInternalUrl, cookieHeader: SLOW_COOKIE }),
      null,
    );
    const unreachable = {
      apiInternalUrl: "http://127.0.0.1:1",
      cookieHeader: VALID_COOKIE,
    };
    assert.equal(await loadSessionUser(unreachable), null);
  });
});

function respondAsTheApi(
  request: IncomingMessage,
  response: ServerResponse,
): void {
  if (request.url !== "/session") {
    response.writeHead(404).end();
    return;
  }
  switch (request.headers.cookie) {
    case VALID_COOKIE:
      response
        .writeHead(200, { "content-type": "application/json" })
        .end(JSON.stringify(ADA));
      return;
    case SLOW_COOKIE:
      setTimeout(
        () => response.writeHead(200).end(JSON.stringify(ADA)),
        SLOWER_THAN_TIMEOUT_MS,
      );
      return;
    case BROKEN_COOKIE:
      response.writeHead(500).end();
      return;
    default:
      response.writeHead(401).end();
  }
}
