import { SignInClient } from "./signin-client"

export default async function SignInPage({
  searchParams,
}: {
  searchParams: Promise<{ callbackUrl?: string }>
}) {
  const { callbackUrl } = await searchParams
  return <SignInClient callbackUrl={callbackUrl || "/onboarding"} />
}
