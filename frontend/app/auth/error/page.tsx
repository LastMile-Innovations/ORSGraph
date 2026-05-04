import Link from "next/link"
import { ShieldAlert } from "lucide-react"
import { AuthFrame } from "@/components/auth/auth-frame"

type AuthErrorPageProps = Omit<PageProps<"/auth/error">, "searchParams"> & {
  searchParams: Promise<{ error?: string }>
}

export default async function AuthErrorPage({
  searchParams,
}: AuthErrorPageProps) {
  const { error } = await searchParams
  return (
    <AuthFrame
      eyebrow="Sign-in issue"
      title="We could not finish signing you in."
      body="The account connection did not complete cleanly. You can try again, use your invite link, or request beta access."
    >
      <section className="rounded-md border border-border bg-card p-5 shadow-sm">
        <div className="flex items-start gap-3">
          <ShieldAlert className="mt-1 h-6 w-6 shrink-0 text-destructive" />
          <div>
            <h2 className="text-lg font-semibold">Authentication error</h2>
            <p className="mt-1 text-sm text-muted-foreground">{error || "Unknown auth provider error"}</p>
          </div>
        </div>
        <div className="mt-5 grid gap-2 sm:grid-cols-2">
          <Link href="/auth/signin" className="inline-flex min-h-10 items-center justify-center rounded-md bg-primary px-3 text-sm font-medium text-primary-foreground hover:bg-primary/90">
            Try again
          </Link>
          <Link href="/auth/request-access" className="inline-flex min-h-10 items-center justify-center rounded-md border border-border px-3 text-sm font-medium hover:bg-muted">
            Request access
          </Link>
        </div>
      </section>
    </AuthFrame>
  )
}
