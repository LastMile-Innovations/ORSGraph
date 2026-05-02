# Cloudflare Authority Cache

This is the fast path for public legal-authority reads. The key idea is simple:
laws and statutes are mostly stable, so the public read surface can be treated as
release-addressed content. Cloudflare handles edge caching, R2 can serve a
precomputed JSON hotset, and Railway remains the live origin fallback.

CaseBuilder, auth, admin, sidebar, document content, and mutations do not use this
path. They stay on `/api/ors/*` or their existing private routes with `no-store`.

## Runtime Config

There is only one optional frontend runtime variable for this feature:

```sh
ORS_AUTHORITY_HOTSET_BASE_URL=https://<r2-custom-domain>/authority-hotset/<encoded-release-id>
```

Leave it unset when no hotset is published. `/api/authority/*` will still work by
falling back to Railway through `ORS_API_BASE_URL`.

The URL must point at the release folder itself. For example, if the generated
folder is:

```text
data/authority-hotset/release%3A2026-05-02/
```

then set:

```sh
ORS_AUTHORITY_HOTSET_BASE_URL=https://authority.example.com/authority-hotset/release%3A2026-05-02
```

Do not add separate release-id, TTL, or edge-mode env vars. TTLs are code defaults
and Cloudflare cache rules can override edge behavior without changing app config.

## Request Flow

Public authority reads go through the same-origin Next route:

```text
browser
  -> /api/authority/*
  -> allowlist + normalize path/query
  -> R2 hotset lookup if ORS_AUTHORITY_HOTSET_BASE_URL is set
  -> Railway API fallback through ORS_API_BASE_URL
```

The gateway emits headers so the active path is visible:

- `x-ors-authority-origin`: `r2-hotset`, `backend`, or `blocked`
- `x-ors-authority-normalized-key`: normalized path and query used for cache identity
- `x-ors-authority-hotset-path`: generated JSON object path inside the release folder
- `x-ors-corpus-release`: release inferred from the hotset URL or returned by origin
- `x-ors-authority-timing-ms`: route handler timing
- `cf-cache-status`: Cloudflare status when present, otherwise local fallback labels

## Public Route Policy

Cacheable/public:

- `/home`
- `/stats`
- `/featured-statutes`
- `/analytics/home`
- `/statutes*`
- `/provisions*`
- `/search` when `q` is present
- `/search/open`
- `/graph/neighborhood`
- `/rules*`
- `/sources*`

Allowed but not hotset-cacheable:

- `/search/suggest`, because it is interactive and changes with every keystroke
- graph reads outside the small hotset scope, where the backend is still allowed

Blocked from `/api/authority/*`:

- `/auth*`
- `/admin*`
- `/ask*`
- `/casebuilder*`
- `/matters*`
- `/qc*`
- `/sidebar*`
- unknown prefixes

Use `/api/ors/*` for private app/API behavior.

## Hotset Build

Build the hotset against Railway or a warmed local API:

```sh
cd frontend
ORS_AUTHORITY_BASE_URL=https://orsgraph-api-production.up.railway.app/api/v1 pnpm hotset:authority
```

Output is written under:

```text
data/authority-hotset/<encoded-release-id>/
```

The release id comes from `data/graph/corpus_release.json` when that ignored
data artifact exists. If it is absent, the builder probes the configured API and
uses the `x-ors-corpus-release` header from `/stats`. The generated
`manifest.json` records source endpoints, normalized keys, object paths, bytes,
and failures.

The initial hotset covers:

- homepage/stats/analytics reads
- statute index
- common direct-open citation lookups
- top keyword/hybrid searches with rerank disabled
- common statute pages/provisions
- small graph neighborhoods
- applicable-rule reads

## Publish To R2

1. Build the hotset.
2. Upload/sync the generated release folder to R2 under `authority-hotset/`.
3. Point `ORS_AUTHORITY_HOTSET_BASE_URL` at that exact release prefix.
4. Redeploy the frontend.

A new corpus release should publish a new R2 prefix. Do not mutate an old release
folder in place; keeping old prefixes immutable prevents stale-law ambiguity.

## Cloudflare Rules

Use Cloudflare in front of Railway and the R2 hotset custom domain.

- Cache `/api/authority/*` at the edge and respect origin `cache-control`.
- Cache the R2 custom-domain hotset path for the same or longer TTL.
- Bypass cache for `/api/ors/*`, `/api/auth/*`, `/api/casebuilder/*`, all non-GET/HEAD requests, and `/api/authority/search/suggest*`.
- Use normal Cloudflare CDN/cache-rule stale-while-revalidate behavior. Do not rely on Worker Cache API `cache.put` for SWR semantics.

Do not use the Railway object-storage bucket as the public hotset origin.
Railway buckets are private storage for app artifacts and uploads; public hotset
reads need a Cloudflare R2 bucket with either a production custom domain or a
temporary development URL.

## Verification

Local static/code checks:

```sh
cd frontend
pnpm exec vitest run lib/authority-hotset.test.ts
pnpm run typecheck
```

Backend header test:

```sh
cargo test -p orsgraph-api routes::authority::tests::authority_headers_include_release_cache_and_cdn_policy
```

Runtime smoke once an API is reachable:

```sh
cd frontend
pnpm perf:authority
```

Expected signs:

- `x-ors-authority-origin: r2-hotset` for hotset hits
- `x-ors-authority-origin: backend` for origin fallback
- `x-ors-authority-origin: blocked` for private/unknown authority-proxy paths
- `x-ors-corpus-release` matches the published release
- `pnpm perf:authority` stays inside the budgets in `scripts/perf-authority.mjs`

## Why Not More Env Vars?

The operational model is intentionally small:

- `ORS_API_BASE_URL` already tells the frontend where Railway origin is.
- `ORS_AUTHORITY_HOTSET_BASE_URL` is enough to opt into R2 hotset reads.
- Release identity is embedded in the hotset URL and in `corpus_release.json`.
- TTL/SWR defaults live in code and can be overridden by Cloudflare rules.
- Private routes are blocked by code, not by deployment convention.
