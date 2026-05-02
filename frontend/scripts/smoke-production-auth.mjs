const frontendUrl = normalizeUrl(process.env.FRONTEND_URL || "https://frontend-production-090c.up.railway.app")
const apiUrl = normalizeUrl(process.env.ORS_API_URL || "https://orsgraph-api-production.up.railway.app/api/v1")
const zitadelUrl = normalizeUrl(process.env.ZITADEL_URL || "https://zitadel-production-ff6c.up.railway.app")

const failures = []

await checkText("ZITADEL health", `${zitadelUrl}/debug/healthz`, {
  status: 200,
  includes: "ok",
})
await checkJson("ZITADEL console env", `${zitadelUrl}/ui/console/assets/environment.json`, {
  status: 200,
  validate: (body) => body.issuer === zitadelUrl && typeof body.clientid === "string" && body.clientid.length > 0,
})
await checkJson("ZITADEL discovery", `${zitadelUrl}/.well-known/openid-configuration`, {
  status: 200,
  validate: (body) => body.issuer === zitadelUrl && body.jwks_uri === `${zitadelUrl}/oauth/v2/keys`,
})
await checkJson("ZITADEL JWKS", `${zitadelUrl}/oauth/v2/keys`, {
  status: 200,
  validate: (body) => Array.isArray(body.keys) && body.keys.length > 0,
})
await checkJson("ZITADEL features auth boundary", `${zitadelUrl}/v2/features/instance`, {
  status: 401,
  validate: (body) => body.message === "auth header missing",
})
await checkJson("frontend health", `${frontendUrl}/api/health`, {
  status: 200,
  validate: (body) => body.ok === true,
})
await checkJson("frontend auth providers", `${frontendUrl}/api/auth/providers`, {
  status: 200,
  validate: (body) => body.zitadel?.id === "zitadel" && body.zitadel?.type === "oauth",
})
await checkRedirect("frontend ZITADEL signin redirect", `${frontendUrl}/api/auth/signin/zitadel?callbackUrl=${encodeURIComponent(`${frontendUrl}/dashboard`)}`, {
  status: 302,
  validateLocation: (location) => {
    const redirect = new URL(location, frontendUrl)
    return redirect.origin === zitadelUrl && redirect.pathname === "/oauth/v2/authorize"
  },
})
await checkJson("API health", `${apiUrl}/health`, {
  status: 200,
  validate: (body) => body.ok === true && body.neo4j === "connected",
})

if (failures.length > 0) {
  console.error("\nProduction auth smoke failures:")
  for (const failure of failures) {
    console.error(`fail ${failure.name}: ${failure.reason}`)
  }
  process.exit(1)
}

console.log(`\nProduction auth smoke passed for ${frontendUrl}, ${apiUrl}, and ${zitadelUrl}`)

async function checkText(name, url, expectation) {
  await check(name, url, expectation, async (response) => response.text())
}

async function checkJson(name, url, expectation) {
  await check(name, url, expectation, async (response) => response.json())
}

async function check(name, url, expectation, readBody) {
  try {
    const response = await fetch(url, { redirect: "manual" })
    const body = await readBody(response)
    if (response.status !== expectation.status) {
      failures.push({ name, reason: `expected ${expectation.status}, got ${response.status}` })
      return
    }
    if (expectation.includes && typeof body === "string" && !body.includes(expectation.includes)) {
      failures.push({ name, reason: `body did not include ${expectation.includes}` })
      return
    }
    if (expectation.validate && !expectation.validate(body)) {
      failures.push({ name, reason: "response body failed validation" })
      return
    }
    console.log(`ok ${response.status} ${name}`)
  } catch (error) {
    failures.push({ name, reason: error instanceof Error ? error.message : String(error) })
  }
}

async function checkRedirect(name, url, expectation) {
  try {
    const response = await fetch(url, { redirect: "manual" })
    const location = response.headers.get("location")
    if (response.status !== expectation.status) {
      failures.push({ name, reason: `expected ${expectation.status}, got ${response.status}` })
      return
    }
    if (!location || !expectation.validateLocation(location)) {
      failures.push({ name, reason: `unexpected redirect ${location || "<none>"}` })
      return
    }
    console.log(`ok ${response.status} ${name} -> ${location}`)
  } catch (error) {
    failures.push({ name, reason: error instanceof Error ? error.message : String(error) })
  }
}

function normalizeUrl(value) {
  return value.replace(/\/$/, "")
}
