import { PendingClient } from "./pending-client"

type PendingPageProps = Omit<PageProps<"/auth/pending">, "searchParams"> & {
  searchParams: Promise<{ callbackUrl?: string }>
}

export default async function PendingPage({
  searchParams,
}: PendingPageProps) {
  const { callbackUrl } = await searchParams
  return <PendingClient callbackUrl={callbackUrl || "/onboarding"} />
}
