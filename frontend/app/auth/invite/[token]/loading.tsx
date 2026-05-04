import { AuthFrame } from "@/components/auth/auth-frame"

export default function InviteLoading() {
  return (
    <AuthFrame
      eyebrow="Beta invite"
      title="Checking your ORSGraph beta invite."
      body="Use the invited account to activate your protected workspace, then create your first CaseBuilder matter."
    >
      <section className="rounded-md border border-border bg-card p-5 shadow-sm">
        <div className="flex items-start gap-3">
          <div className="mt-1 h-6 w-6 shrink-0 animate-pulse rounded bg-muted" />
          <div className="min-w-0 flex-1">
            <div className="h-5 w-36 animate-pulse rounded bg-muted" />
            <div className="mt-3 h-4 w-full animate-pulse rounded bg-muted" />
            <div className="mt-2 h-4 w-3/4 animate-pulse rounded bg-muted" />
          </div>
        </div>
        <div className="mt-5 h-11 w-full animate-pulse rounded-md bg-muted" />
      </section>
    </AuthFrame>
  )
}
