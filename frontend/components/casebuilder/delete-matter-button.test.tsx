import { render, screen, waitFor } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { beforeEach, describe, expect, it, vi } from "vitest"
import { DeleteMatterButton } from "./delete-matter-button"

const router = {
  replace: vi.fn(),
  refresh: vi.fn(),
}

vi.mock("next/navigation", () => ({
  useRouter: () => router,
}))

const deleteMatter = vi.fn()

vi.mock("@/lib/casebuilder/api", () => ({
  deleteMatter: (...args: unknown[]) => deleteMatter(...args),
}))

describe("DeleteMatterButton", () => {
  beforeEach(() => {
    router.replace.mockReset()
    router.refresh.mockReset()
    deleteMatter.mockReset()
    window.localStorage.clear()
    deleteMatter.mockResolvedValue({ data: { deleted: true } })
  })

  it("requires the matter name before hard-deleting and redirecting", async () => {
    const user = userEvent.setup()
    window.localStorage.setItem("casebuilder:ask:matter:intake-test", "thread")

    render(<DeleteMatterButton matter={{ matter_id: "matter:intake-test", name: "Intake Test" }} />)

    await user.click(screen.getByRole("button", { name: /^delete matter$/i }))
    expect(screen.getByRole("button", { name: /delete permanently/i })).toBeDisabled()

    await user.type(screen.getByLabelText(/type intake test to confirm/i), "Intake Test")
    await user.click(screen.getByRole("button", { name: /delete permanently/i }))

    await waitFor(() => {
      expect(deleteMatter).toHaveBeenCalledWith("matter:intake-test")
      expect(router.replace).toHaveBeenCalledWith("/casebuilder")
      expect(router.refresh).toHaveBeenCalled()
    })
    expect(window.localStorage.getItem("casebuilder:ask:matter:intake-test")).toBeNull()
  })

  it("keeps the dialog open when the delete endpoint fails", async () => {
    const user = userEvent.setup()
    deleteMatter.mockResolvedValue({ data: null, error: "Delete failed" })

    render(<DeleteMatterButton matter={{ matter_id: "matter:intake-test", name: "Intake Test" }} />)

    await user.click(screen.getByRole("button", { name: /^delete matter$/i }))
    await user.type(screen.getByLabelText(/type intake test to confirm/i), "Intake Test")
    await user.click(screen.getByRole("button", { name: /delete permanently/i }))

    expect(await screen.findByRole("alert")).toHaveTextContent("Delete failed")
    expect(router.replace).not.toHaveBeenCalled()
  })
})
