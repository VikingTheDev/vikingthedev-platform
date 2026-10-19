import type { APIRoute } from "astro";
import { GIT_SHA } from "astro:env/server";

export const prerender = false;

export const GET: APIRoute = () =>
  Response.json(
    { status: "ok", sha: GIT_SHA },
    { headers: { "Cache-Control": "no-store" } },
  );
