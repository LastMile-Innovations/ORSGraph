import "server-only"

export function orsBackendApiBaseUrl() {
  return (
    process.env.ORS_API_BASE_URL ||
    process.env.NEXT_PUBLIC_ORS_API_BASE_URL ||
    "http://localhost:8080/api/v1"
  ).replace(/\/$/, "")
}
