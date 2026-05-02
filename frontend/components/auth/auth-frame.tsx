import Link from "next/link"
import { Landmark, ShieldCheck } from "lucide-react"

export function AuthFrame({
  eyebrow,
  title,
  body,
  children,
}: {
  eyebrow: string
  title: string
  body: string
  children: React.ReactNode
}) {
  return (
    <main className="min-h-screen bg-background text-foreground">
      <div className="mx-auto flex min-h-screen w-full max-w-6xl flex-col px-4 py-6 sm:px-6 lg:px-8">
        <header className="flex items-center justify-between gap-3">
          <Link href="/" className="flex min-w-0 items-center gap-3 rounded-md outline-none focus-visible:ring-2 focus-visible:ring-ring/60">
            <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-md bg-primary text-primary-foreground">
              <Landmark className="h-5 w-5" />
            </span>
            <span className="font-mono text-sm font-semibold tracking-normal">ORSGraph</span>
          </Link>
          <div className="hidden items-center gap-2 rounded-md border border-border px-3 py-2 text-xs text-muted-foreground sm:flex">
            <ShieldCheck className="h-3.5 w-3.5 text-primary" />
            Invite-only beta
          </div>
        </header>

        <section className="grid flex-1 gap-10 py-12 lg:grid-cols-[0.92fr_1.08fr] lg:items-center">
          <div className="max-w-xl">
            <div className="font-mono text-xs uppercase tracking-normal text-primary">{eyebrow}</div>
            <h1 className="mt-4 text-balance text-4xl font-semibold tracking-normal sm:text-5xl">{title}</h1>
            <p className="mt-5 text-pretty text-base leading-7 text-muted-foreground sm:text-lg">{body}</p>
            <div className="mt-8 grid gap-3 text-sm text-muted-foreground sm:grid-cols-3">
              <Proof label="Private matter spaces" />
              <Proof label="Source-backed work" />
              <Proof label="Plain-language intake" />
            </div>
          </div>
          <div className="w-full">{children}</div>
        </section>
      </div>
    </main>
  )
}

function Proof({ label }: { label: string }) {
  return (
    <div className="flex min-h-14 items-center gap-2 rounded-md border border-border bg-card px-3 py-2">
      <ShieldCheck className="h-4 w-4 shrink-0 text-success" />
      <span>{label}</span>
    </div>
  )
}
