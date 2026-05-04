"use client"

import Link, { useLinkStatus } from "next/link"
import { useState, type ComponentProps } from "react"
import { cn } from "@/lib/utils"

type HoverPrefetchLinkProps = Omit<ComponentProps<typeof Link>, "prefetch"> & {
  pendingIndicatorClassName?: string
}

export function HoverPrefetchLink({
  children,
  onFocus,
  onMouseEnter,
  pendingIndicatorClassName,
  ...props
}: HoverPrefetchLinkProps) {
  const [active, setActive] = useState(false)

  return (
    <Link
      {...props}
      prefetch={active ? null : false}
      onFocus={(event) => {
        setActive(true)
        onFocus?.(event)
      }}
      onMouseEnter={(event) => {
        setActive(true)
        onMouseEnter?.(event)
      }}
    >
      {children}
      {pendingIndicatorClassName ? <LinkPendingIndicator className={pendingIndicatorClassName} /> : null}
    </Link>
  )
}

function LinkPendingIndicator({ className }: { className: string }) {
  const { pending } = useLinkStatus()

  return (
    <span
      aria-hidden="true"
      className={cn(
        "pointer-events-none inline-block h-1.5 w-1.5 shrink-0 rounded-full bg-primary opacity-0 transition-opacity delay-100",
        pending && "animate-pulse opacity-100",
        className,
      )}
    />
  )
}
