const baseUrl = (process.env.SMOKE_BASE_URL || process.argv[2] || "http://localhost:3000").replace(/\/$/, "")

const routes = [
  { path: "/" },
  { path: "/search" },
  { path: "/search?q=90.300" },
  { path: "/search?q=chapter%2090%20habitability" },
  { path: "/ask" },
  { path: "/graph" },
  { path: "/qc" },
  { path: "/admin" },
  { path: "/statutes" },
  { path: "/statutes?chapter=3" },
  { path: "/statutes/or:ors:3.130" },
  { path: "/statutes/or:ors:3.010" },
  { path: "/statutes/or:ors:8.610" },
  { path: "/statutes/or:ors:3.275" },
  { path: "/casebuilder" },
  { path: "/casebuilder/new" },
  { path: "/casebuilder/matters/smith-abc" },
  { path: "/casebuilder/matters/matter%3Asmith-abc" },
  { path: "/casebuilder/matters/smith-abc/documents" },
  { path: "/casebuilder/matters/smith-abc/documents/doc%3Acomplaint" },
  { path: "/casebuilder/matters/smith-abc/facts" },
  { path: "/casebuilder/matters/smith-abc/timeline" },
  { path: "/casebuilder/matters/smith-abc/claims" },
  { path: "/casebuilder/matters/smith-abc/evidence" },
  { path: "/casebuilder/matters/smith-abc/deadlines" },
  { path: "/casebuilder/matters/smith-abc/complaint" },
  { path: "/casebuilder/matters/smith-abc/complaint/editor" },
  { path: "/casebuilder/matters/smith-abc/complaint/outline" },
  { path: "/casebuilder/matters/smith-abc/complaint/claims" },
  { path: "/casebuilder/matters/smith-abc/complaint/evidence" },
  { path: "/casebuilder/matters/smith-abc/complaint/qc" },
  { path: "/casebuilder/matters/smith-abc/complaint/preview" },
  { path: "/casebuilder/matters/smith-abc/complaint/export" },
  { path: "/casebuilder/matters/smith-abc/work-products" },
  { path: "/casebuilder/matters/smith-abc/work-products/new" },
  { path: "/casebuilder/matters/smith-abc/work-products/work-product%3Amatter%3Asmith-abc%3Aanswer-demo" },
  { path: "/casebuilder/matters/smith-abc/work-products/work-product%3Amatter%3Asmith-abc%3Aanswer-demo/editor" },
  { path: "/casebuilder/matters/smith-abc/work-products/work-product%3Amatter%3Asmith-abc%3Aanswer-demo/qc" },
  { path: "/casebuilder/matters/smith-abc/work-products/work-product%3Amatter%3Asmith-abc%3Aanswer-demo/preview" },
  { path: "/casebuilder/matters/smith-abc/work-products/work-product%3Amatter%3Asmith-abc%3Aanswer-demo/export" },
  { path: "/casebuilder/matters/smith-abc/work-products/work-product%3Amatter%3Asmith-abc%3Aanswer-demo/history" },
  { path: "/casebuilder/matters/smith-abc/drafts" },
  { path: "/casebuilder/matters/smith-abc/drafts/draft%3Aanswer-v3" },
  { path: "/casebuilder/matters/smith-abc/ask" },
  { path: "/casebuilder/matters/smith-abc/authorities" },
  { path: "/casebuilder/matters/smith-abc/tasks" },
  { path: "/matters", redirectTo: "/casebuilder" },
  { path: "/matters/new", redirectTo: "/casebuilder/new" },
  { path: "/matters/smith-abc", redirectTo: "/casebuilder/matters/smith-abc" },
  { path: "/matters/smith-abc/documents", redirectTo: "/casebuilder/matters/smith-abc/documents" },
]

const failures = []
const routeHtml = new Map()

for (const route of routes) {
  const url = `${baseUrl}${route.path}`
  try {
    const response = await fetch(url, { redirect: "manual" })
    const location = response.headers.get("location")
    const html = await response.text().catch(() => "")
    if (route.path === "/" && response.status >= 200 && response.status < 300) {
      routeHtml.set("/", html)
    }
    const isOk = response.status >= 200 && response.status < 300
    const expectedRedirect = route.redirectTo ? new URL(route.redirectTo, baseUrl) : null
    const isExpectedRedirect = Boolean(
      expectedRedirect &&
        response.status >= 300 &&
        response.status < 400 &&
        location &&
        new URL(location, baseUrl).pathname === expectedRedirect.pathname,
    )
    const hasNext404 = html.includes("This page could not be found")

    if ((!isOk && !isExpectedRedirect) || (!expectedRedirect && location) || hasNext404) {
      failures.push({
        route: route.path,
        status: response.status,
        redirect: location,
        reason: hasNext404 ? "default Next 404 body" : "non-2xx or redirect",
      })
    } else {
      console.log(`ok ${response.status} ${route.path}${location ? ` -> ${location}` : ""}`)
    }
  } catch (error) {
    failures.push({
      route: route.path,
      status: "fetch-error",
      redirect: "",
      reason: error instanceof Error ? error.message : String(error),
    })
  }
}

await smokeHomeLinks()

if (failures.length > 0) {
  console.error("\nRoute smoke failures:")
  for (const failure of failures) {
    console.error(
      `fail ${failure.status} ${failure.route}${failure.redirect ? ` -> ${failure.redirect}` : ""} (${failure.reason})`,
    )
  }
  process.exit(1)
}

console.log(`\n${routes.length} route smoke checks passed against ${baseUrl}`)

async function smokeHomeLinks() {
  const html = routeHtml.get("/") || await fetch(`${baseUrl}/`).then((response) => response.text())
  const localHrefs = extractLocalHrefs(html)

  for (const href of localHrefs) {
    const url = new URL(href, baseUrl)
    try {
      const response = await fetch(url, { redirect: "manual" })
      const location = response.headers.get("location")
      const body = await response.text().catch(() => "")
      const hasNext404 = body.includes("This page could not be found")
      const isOk = response.status >= 200 && response.status < 300

      if (!isOk || location || hasNext404) {
        failures.push({
          route: `home link ${href}`,
          status: response.status,
          redirect: location,
          reason: hasNext404 ? "default Next 404 body" : "home link failed",
        })
      } else {
        console.log(`ok ${response.status} home link ${href}`)
      }
    } catch (error) {
      failures.push({
        route: `home link ${href}`,
        status: "fetch-error",
        redirect: "",
        reason: error instanceof Error ? error.message : String(error),
      })
    }
  }
}

function extractLocalHrefs(html) {
  const hrefs = new Set()
  for (const match of html.matchAll(/\shref="([^"]+)"/g)) {
    const raw = decodeHtml(match[1])
    if (!raw || raw.startsWith("#") || raw.startsWith("mailto:") || raw.startsWith("tel:")) continue

    const url = new URL(raw, baseUrl)
    if (url.origin !== new URL(baseUrl).origin) continue
    if (url.pathname.startsWith("/_next") || url.pathname === "/favicon.ico") continue

    hrefs.add(`${url.pathname}${url.search}`)
  }
  return [...hrefs].sort()
}

function decodeHtml(value) {
  return value
    .replaceAll("&amp;", "&")
    .replaceAll("&quot;", "\"")
    .replaceAll("&#x27;", "'")
}
