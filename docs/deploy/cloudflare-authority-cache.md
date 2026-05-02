# Cloudflare Authority Cache

Use Cloudflare in front of Railway as the public authority-read accelerator. Railway remains the origin; R2 stores precomputed release hotsets.

## Hotset Publish

1. Build against the Railway API or a warmed local API:
   ```sh
   cd frontend
   ORS_AUTHORITY_BASE_URL=https://orsgraph-api-production.up.railway.app/api/v1 pnpm hotset:authority
   ```
2. Sync `data/authority-hotset/<encoded-release-id>/` to R2.
3. Set one frontend env to the full release prefix:
   - `ORS_AUTHORITY_HOTSET_BASE_URL=https://<r2-custom-domain>/authority-hotset/<encoded-release-id>`

## Cache Rules

- Cache `/api/authority/*` at the edge using the normalized URL. Respect origin `cache-control`.
- Cache the R2 custom-domain hotset path for the same or longer TTL than `/api/authority/*`.
- Bypass cache for `/api/ors/*`, `/api/auth/*`, `/api/casebuilder/*`, `/api/authority/search/suggest*`, and all non-GET/HEAD requests.
- Keep stale-while-revalidate on normal CDN/cache-rule behavior. Do not rely on Worker Cache API `cache.put` for SWR semantics.

## Verification

- `pnpm perf:authority` should report p95 within the budgets in `scripts/perf-authority.mjs`.
- Authority responses should include `x-ors-authority-origin`, `x-ors-authority-normalized-key`, `x-ors-corpus-release`, `cf-cache-status`, and `x-ors-authority-timing-ms`.
- A new corpus release should publish a new hotset release prefix rather than mutating an old prefix in place.
