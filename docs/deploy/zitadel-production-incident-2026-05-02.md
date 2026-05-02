# ZITADEL Production Incident - 2026-05-02

## Summary

On 2026-05-02, production ZITADEL returned HTTP 500 for console and feature
requests on `https://zitadel-production-ff6c.up.railway.app`.

The visible error was:

```text
json: cannot unmarshal bool into Go struct field Features.login_v2 of type feature.LoginV2
```

## Cause

The eventstore source event for `feature.instance.login_v2.set` had the expected
object payload:

```json
{"Value":{"required":true}}
```

The `projections.instance_features5` projection row for `login_v2` instead held
the stale JSON boolean:

```json
false
```

That projection shape is incompatible with ZITADEL v4.14.0, which expects
`login_v2` to unmarshal as an object.

## Repair

The projection row was repaired directly in `zitadel-postgres` after confirming
the matching source event:

```sql
update projections.instance_features5 f
set value = '{"required": true}'::jsonb,
    change_date = now()
where f.instance_id = '371183997394405742'
  and f.key = 'login_v2'
  and f.value = 'false'::jsonb
  and exists (
    select 1
    from eventstore.events2 e
    where e.instance_id = f.instance_id
      and e.aggregate_id = f.instance_id
      and e.event_type = 'feature.instance.login_v2.set'
      and e.sequence = f.sequence
      and e.payload = '{"Value":{"required":true}}'::jsonb
  );
```

The transaction updated exactly one row.

## Verification

After the repair:

- ZITADEL health returned `ok`.
- OIDC discovery returned HTTP 200.
- JWKS returned HTTP 200.
- Console environment returned HTTP 200.
- `GET /v2/features/instance` returned the expected unauthenticated HTTP 401
  instead of HTTP 500.
- Railway logs showed successful instance resolution for the repaired instance.

## Follow-Up

- Keep watching for `login_v2` projection regressions after ZITADEL redeploys or
  upgrades.
- Finish ZITADEL application bootstrap for ORSGraph before enabling API audience
  validation in production.
- Do not enable `ORS_AUTH_ENABLED=true` until `frontend` has a real ZITADEL OIDC
  client ID/secret and `orsgraph-api` has the matching `ORS_AUTH_AUDIENCE`.
