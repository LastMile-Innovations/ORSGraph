import { fireEvent, render, screen } from "@testing-library/react"
import { beforeEach, describe, expect, it, vi } from "vitest"
import type { ComponentProps } from "react"
import type { StatuteIdentity } from "@/lib/types"
import { StatuteIndexClient } from "./statute-index-client"

const pushMock = vi.fn()

vi.mock("next/navigation", () => ({
  useRouter: () => ({ push: pushMock }),
}))

function statute(overrides: Partial<StatuteIdentity> = {}): StatuteIdentity {
  return {
    canonical_id: "or:ors:90.320",
    citation: "ORS 90.320",
    title: "Landlord to maintain premises in habitable condition",
    jurisdiction: "Oregon",
    corpus: "ORS",
    chapter: "90",
    status: "active",
    edition: 2025,
    ...overrides,
  }
}

function renderIndex(props: Partial<ComponentProps<typeof StatuteIndexClient>> = {}) {
  return render(
    <StatuteIndexClient
      statutes={[statute()]}
      total={1}
      limit={60}
      offset={0}
      query=""
      chapter="90"
      status="all"
      dataSource="live"
      {...props}
    />,
  )
}

describe("StatuteIndexClient", () => {
  beforeEach(() => {
    pushMock.mockReset()
  })

  it("keeps draft filters synced with URL-derived props", () => {
    const { rerender } = renderIndex()

    expect(screen.getByLabelText("Citation, title, or canonical ID")).toHaveValue("")
    expect(screen.getByPlaceholderText("Chapter")).toHaveValue("90")

    rerender(
      <StatuteIndexClient
        statutes={[statute({ chapter: "1", citation: "ORS 1.001", canonical_id: "or:ors:1.001" })]}
        total={1}
        limit={120}
        offset={0}
        query="court"
        chapter="1"
        status="active"
        dataSource="live"
      />,
    )

    expect(screen.getByLabelText("Citation, title, or canonical ID")).toHaveValue("court")
    expect(screen.getByPlaceholderText("Chapter")).toHaveValue("1")
    expect(screen.getByLabelText("Filter by status")).toHaveValue("active")
    expect(screen.getByLabelText("Rows per page")).toHaveValue("120")
  })

  it("clears visible filters and routes back to the unfiltered directory", () => {
    renderIndex({ query: "rent", chapter: "90", status: "active", limit: 120 })

    fireEvent.click(screen.getByRole("button", { name: "Clear" }))

    expect(pushMock).toHaveBeenCalledWith("/statutes")
    expect(screen.getByLabelText("Citation, title, or canonical ID")).toHaveValue("")
    expect(screen.getByPlaceholderText("Chapter")).toHaveValue("")
    expect(screen.getByLabelText("Filter by status")).toHaveValue("all")
    expect(screen.getByLabelText("Rows per page")).toHaveValue("60")
  })

  it("opens exact ORS citations directly from the directory form", () => {
    renderIndex({ chapter: "" })

    fireEvent.change(screen.getByLabelText("Citation, title, or canonical ID"), {
      target: { value: "ORS 90.320" },
    })
    fireEvent.click(screen.getByRole("button", { name: "Open / filter" }))

    expect(pushMock).toHaveBeenCalledWith("/statutes/or%3Aors%3A90.320")
  })
})
