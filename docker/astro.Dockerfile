# syntax=docker/dockerfile:1
# One image per Astro app: docker build -f docker/astro.Dockerfile --build-arg APP=<app> --build-arg GIT_SHA=<sha> .

FROM node:24.22.0-trixie-slim@sha256:7f486cabe892bff8fef04c0f7da8dba84a560be3e4e2740b7a86afc57ebec315 AS builder
ARG APP
# Builds never send Astro's anonymous usage data (step 7b gate).
ENV PNPM_HOME=/pnpm ASTRO_TELEMETRY_DISABLED=1
WORKDIR /build
RUN corepack enable
COPY package.json ./
# Fetches the pnpm version and hash pinned in packageManager, in its own cached layer.
RUN corepack install
COPY pnpm-lock.yaml pnpm-workspace.yaml ./
COPY apps/${APP}/package.json apps/${APP}/
COPY packages/ packages/
RUN --mount=type=cache,id=pnpm-store,target=/pnpm/store \
    pnpm config set store-dir /pnpm/store && \
    pnpm install --frozen-lockfile --filter "@vikingthedev/${APP}..."
COPY apps/${APP}/ apps/${APP}/
# The SHA is baked into the server bundle at build time (astro:env), like the API's.
ARG GIT_SHA=dev
RUN pnpm --filter "@vikingthedev/${APP}" build
# deploy copies the built dist/ along with the production node_modules.
RUN --mount=type=cache,id=pnpm-store,target=/pnpm/store \
    pnpm --filter "@vikingthedev/${APP}" deploy --prod /deploy

FROM gcr.io/distroless/nodejs24-debian13:nonroot@sha256:59c479c9b8c123e66047315c968aa2d35e31ae5149c856e4110a79c8b3563f0b
WORKDIR /app
COPY --from=builder /deploy/package.json ./
COPY --from=builder /deploy/node_modules ./node_modules
COPY --from=builder /deploy/dist ./dist
ENV NODE_ENV=production HOST=0.0.0.0 PORT=4321
EXPOSE 4321
CMD ["dist/server/entry.mjs"]
