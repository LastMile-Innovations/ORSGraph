const baseUrl = (process.env.SMOKE_BASE_URL || process.argv[2] || "http://localhost:3000").replace(/\/$/, "")

const routes = [
  { path: "/" },
  { path: "/search" },
  { path: "/search?q=90.300" },
  { path: "/search?q=chapter%2090%20habitability" },
  { path: "/ask" },
  { path: "/graph" },
  { path: "/qc" },
  { path: "/statutes" },
  { path: "/statutes/or:ors:3.130" },
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

for (const route of routes) {
  const url = `${baseUrl}${route.path}`
  try {
    const response = await fetch(url, { redirect: "manual" })
    const location = response.headers.get("location")
    const html = await response.text().catch(() => "")
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
