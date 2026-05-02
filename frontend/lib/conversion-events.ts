"use client"

import { track } from "@vercel/analytics"

type ConversionEvent =
  | "landing_cta_click"
  | "access_request_submitted"
  | "invite_accepted"
  | "sign_in_started"
  | "first_matter_started"
  | "first_matter_created"

export function trackConversionEvent(event: ConversionEvent, properties: Record<string, string | number | boolean> = {}) {
  try {
    track(event, properties)
  } catch {
    // Analytics must never block auth or legal-work intake.
  }
}
