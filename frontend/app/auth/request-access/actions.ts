"use server"

import { revalidatePath } from "next/cache"
import { orsBackendApiBaseUrl } from "@/lib/ors-backend-api-url"

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
  const email = formValue(formData, "email").toLowerCase()
  const situation = formValue(formData, "situation_type")
  const urgency = formValue(formData, "deadline_urgency")
  const jurisdiction = formValue(formData, "jurisdiction")
  const note = formValue(formData, "note")

  if (!email || !email.includes("@")) {
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
        error: typeof body.error === "string" ? body.error : `Request failed: ${response.status}`,
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
  } catch (error) {
    return {
      ok: false,
      error: error instanceof Error ? error.message : "Could not submit access request.",
      situation,
      urgency,
    }
  }
}

function formValue(formData: FormData, name: string) {
  const value = formData.get(name)
  return typeof value === "string" ? value.trim() : ""
}
