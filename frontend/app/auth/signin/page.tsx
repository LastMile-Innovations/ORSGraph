import { SignInClient } from "./signin-client"
import { safeCallbackHref } from "@/lib/navigation-safety"

type SignInPageProps = Omit<PageProps<"/auth/signin">, "searchParams"> & {
  searchParams: Promise<{ callbackUrl?: string }>
}

export default async function SignInPage({
  searchParams,
}: SignInPageProps) {
  const { callbackUrl } = await searchParams
  return <SignInClient safeCallbackUrl={safeCallbackHref(callbackUrl)} />
}
