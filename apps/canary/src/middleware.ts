import { API_INTERNAL_URL } from "astro:env/server";
import { defineMiddleware } from "astro:middleware";
import { loadSessionUser } from "@vikingthedev/session";

export const onRequest = defineMiddleware(async (context, next) => {
  context.locals.user = context.isPrerendered
    ? null
    : await loadSessionUser({
        apiInternalUrl: API_INTERNAL_URL,
        cookieHeader: context.request.headers.get("cookie"),
      });
  return next();
});
