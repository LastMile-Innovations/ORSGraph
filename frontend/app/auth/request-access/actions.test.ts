import { afterEach, describe, expect, it, vi } from "vitest"

const mocks = vi.hoisted(() => ({
  revalidatePath: vi.fn(),
}))

vi.mock("next/cache", () => ({
  revalidatePath: mocks.revalidatePath,
}))

import { submitAccessRequest } from "./actions"

function accessForm(overrides: Record<string, string> = {}) {
  const formData = new FormData()
  formData.set("email", overrides.email ?? "USER@example.COM")
  formData.set("situation_type", overrides.situation_type ?? "I need to build a complaint")
  formData.set("deadline_urgency", overrides.deadline_urgency ?? "Deadline this month")
  formData.set("jurisdiction", overrides.jurisdiction ?? "Oregon")
  formData.set("note", overrides.note ?? "Need help organizing claims.")
  return formData
}

describe("submitAccessRequest", () => {
  afterEach(() => {
    vi.restoreAllMocks()
    vi.unstubAllGlobals()
    mocks.revalidatePath.mockReset()
  })

  it("rejects invalid emails before calling the backend", async () => {
    const fetchMock = vi.fn()
    vi.stubGlobal("fetch", fetchMock)

    const state = await submitAccessRequest({ ok: false }, accessForm({ email: "not-an-email" }))

    expect(state).toEqual({
      ok: false,
      error: "Enter a valid email address.",
      situation: "I need to build a complaint",
      urgency: "Deadline this month",
    })
    expect(fetchMock).not.toHaveBeenCalled()
  })

  it("normalizes and bounds submitted data before forwarding it", async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ message: "Received" }),
    })
    vi.stubGlobal("fetch", fetchMock)

    const state = await submitAccessRequest({ ok: false }, accessForm({ note: "x".repeat(1300) }))

    expect(state.ok).toBe(true)
    expect(fetchMock).toHaveBeenCalledTimes(1)
    const init = fetchMock.mock.calls[0][1] as RequestInit
    expect(JSON.parse(String(init.body))).toMatchObject({
      email: "user@example.com",
      situation_type: "I need to build a complaint",
      deadline_urgency: "Deadline this month",
      jurisdiction: "Oregon",
      note: "x".repeat(1200),
    })
    expect(mocks.revalidatePath).toHaveBeenCalledWith("/admin/auth")
  })

  it("does not return raw upstream error details", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({
      ok: false,
      status: 500,
      json: async () => ({ error: "database credentials leaked here" }),
    }))

    const state = await submitAccessRequest({ ok: false }, accessForm())

    expect(state).toMatchObject({
      ok: false,
      error: "Could not submit access request.",
    })
  })
})
