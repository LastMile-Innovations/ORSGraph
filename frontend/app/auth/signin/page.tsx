import { SignInClient } from "./signin-client"

type SignInPageProps = Omit<PageProps<"/auth/signin">, "searchParams"> & {
  searchParams: Promise<{ callbackUrl?: string }>
}

export default async function SignInPage({
  searchParams,
}: SignInPageProps) {
  const { callbackUrl } = await searchParams
  return <SignInClient callbackUrl={callbackUrl || "/onboarding"} />
}
