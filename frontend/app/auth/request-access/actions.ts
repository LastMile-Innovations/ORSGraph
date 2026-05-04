"use server"

import { revalidatePath } from "next/cache"
import { orsBackendApiBaseUrl } from "@/lib/ors-backend-api-url"

const MAX_EMAIL_LENGTH = 254
const MAX_SITUATION_LENGTH = 80
const MAX_URGENCY_LENGTH = 80
const MAX_JURISDICTION_LENGTH = 120
const MAX_NOTE_LENGTH = 1200
const EMAIL_PATTERN = /^[^\s@]+@[^\s@]+\.[^\s@]+$/

export interface RequestAccessActionState {
  ok: boolean
  message?: string
  error?: string
  situation?: string
  urgency?: string
}

export async function submitAccessRequest(
  _previous: RequestAccessActionState,
  formData: FormData,
): Promise<RequestAccessActionState> {
  const email = normalizeEmail(formData)
  const situation = boundedFormValue(formData, "situation_type", MAX_SITUATION_LENGTH)
  const urgency = boundedFormValue(formData, "deadline_urgency", MAX_URGENCY_LENGTH)
  const jurisdiction = boundedFormValue(formData, "jurisdiction", MAX_JURISDICTION_LENGTH)
  const note = boundedFormValue(formData, "note", MAX_NOTE_LENGTH)

  if (!email) {
    return { ok: false, error: "Enter a valid email address.", situation, urgency }
  }

  if (!situation || !urgency) {
    return { ok: false, error: "Choose what you need and how urgent it is.", situation, urgency }
  }

  try {
    const response = await fetch(`${orsBackendApiBaseUrl()}/auth/access-request`, {
      method: "POST",
      cache: "no-store",
      headers: {
        "Content-Type": "application/json",
        Accept: "application/json",
      },
      body: JSON.stringify({
        email,
        situation_type: situation,
        deadline_urgency: urgency,
        jurisdiction,
        note,
      }),
    })
    const body = (await response.json().catch(() => ({}))) as { error?: unknown; message?: unknown }

    if (!response.ok) {
      return {
        ok: false,
        error: response.status === 400 ? "Check the request fields and try again." : "Could not submit access request.",
        situation,
        urgency,
      }
    }

    revalidatePath("/admin/auth")

    return {
      ok: true,
      message: typeof body.message === "string" ? body.message : "Request received.",
      situation,
      urgency,
    }
  } catch {
    return {
      ok: false,
      error: "Could not submit access request.",
      situation,
      urgency,
    }
  }
}

function normalizeEmail(formData: FormData) {
  const email = boundedFormValue(formData, "email", MAX_EMAIL_LENGTH).toLowerCase()
  return EMAIL_PATTERN.test(email) ? email : ""
}

function boundedFormValue(formData: FormData, name: string, maxLength: number) {
  const value = formData.get(name)
  return typeof value === "string" ? value.trim().slice(0, maxLength) : ""
}
