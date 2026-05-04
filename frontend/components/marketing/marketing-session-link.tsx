"use client"

import Link from "next/link"
import { useSession } from "next-auth/react"
import { forwardRef, type ReactNode } from "react"
import type { ComponentPropsWithoutRef, MouseEvent } from "react"
import { ArrowRight, GitBranch } from "lucide-react"
import { trackConversionEvent, type ConversionEvent } from "@/lib/conversion-events"

type MarketingSessionLinkProps = Omit<ComponentPropsWithoutRef<typeof Link>, "href" | "children" | "onClick"> & {
  signedInHref: string
  signedOutHref: string
  signedInChildren: ReactNode
  signedOutChildren: ReactNode
  signedOutTrackingEvent?: ConversionEvent
  signedOutTrackingProperties?: Record<string, string | number | boolean>
  icon?: "arrow" | "branch"
  hideWhenSignedIn?: boolean
  onClick?: ComponentPropsWithoutRef<typeof Link>["onClick"]
}

export const MarketingSessionLink = forwardRef<HTMLAnchorElement, MarketingSessionLinkProps>(
  function MarketingSessionLink(
    {
      signedInHref,
      signedOutHref,
      signedInChildren,
      signedOutChildren,
      signedOutTrackingEvent,
      signedOutTrackingProperties,
      icon,
      hideWhenSignedIn,
      onClick,
      ...props
    },
    ref,
  ) {
    const session = useSession()
    const isSignedIn = session.status === "authenticated"
    const Icon = icon === "arrow" ? ArrowRight : icon === "branch" ? GitBranch : null

    if (isSignedIn && hideWhenSignedIn) return null

    function handleClick(event: MouseEvent<HTMLAnchorElement>) {
      onClick?.(event)
      if (!event.defaultPrevented && !isSignedIn && signedOutTrackingEvent) {
        trackConversionEvent(signedOutTrackingEvent, signedOutTrackingProperties)
      }
    }

    return (
      <Link
        ref={ref}
        href={isSignedIn ? signedInHref : signedOutHref}
        onClick={handleClick}
        {...props}
      >
        {isSignedIn ? signedInChildren : signedOutChildren}
        {Icon ? <Icon className="h-4 w-4" /> : null}
      </Link>
    )
  },
)
