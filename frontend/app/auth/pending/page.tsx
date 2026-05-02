import { PendingClient } from "./pending-client"

export default async function PendingPage({
  searchParams,
}: {
  searchParams: Promise<{ callbackUrl?: string }>
}) {
  const { callbackUrl } = await searchParams
  return <PendingClient callbackUrl={callbackUrl || "/onboarding"} />
}
