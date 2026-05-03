import { fireEvent, render, screen } from "@testing-library/react"
import { beforeEach, describe, expect, it, vi } from "vitest"
import type { DataState } from "@/lib/data-state"
import type { SidebarData, SidebarStatute } from "@/lib/api"
import { LeftRail } from "./left-rail"
import { MobileLeftRailSheet } from "./mobile-left-rail-sheet"

const pushMock = vi.fn()
let pathname = "/statutes"
let searchParams = new URLSearchParams()

vi.mock("next/navigation", () => ({
  usePathname: () => pathname,
  useRouter: () => ({ push: pushMock }),
  useSearchParams: () => searchParams,
}))

function statute(index: number): SidebarStatute {
  return {
    canonical_id: `or:ors:3.${String(index).padStart(3, "0")}`,
    citation: `ORS 3.${String(index).padStart(3, "0")}`,
    title: `Section ${index}`,
    chapter: "3",
    status: "active",
    edition_year: 2025,
  }
}

function sidebarState(): DataState<SidebarData | null> {
  const items = Array.from({ length: 8 }, (_, index) => statute(index + 1))

  return {
    source: "live",
    data: {
      corpus: {
        jurisdiction: "Oregon",
        corpus: "ORS",
        edition_year: 2025,
        total_statutes: 10,
        chapters: [{
          chapter: "3",
          label: "Chapter 3",
          count: 10,
          items,
        }],
      },
      saved_searches: [{
        saved_search_id: "saved-search:user-a:tenant",
        query: "tenant",
        results: 4,
        created_at: "1",
        updated_at: "2",
      }],
      saved_statutes: [],
      recent_statutes: [],
      active_matter: null,
      updated_at: "2",
    },
  }
}

describe("LeftRail", () => {
  beforeEach(() => {
    pushMock.mockReset()
    pathname = "/statutes"
    searchParams = new URLSearchParams()
  })

  it("routes chapter overflow to the statute directory", () => {
    const handleNavigate = vi.fn()
    render(<LeftRail initialState={sidebarState()} onNavigate={handleNavigate} />)

    const moreLink = screen.getByText("2 more").closest("a")

    expect(moreLink).toHaveAttribute("href", "/statutes?chapter=3")
    moreLink?.addEventListener("click", (event) => event.preventDefault())
    fireEvent.click(moreLink!)
    expect(handleNavigate).toHaveBeenCalledTimes(1)
  })

  it("calls onNavigate when a saved search link is opened", () => {
    const handleNavigate = vi.fn()
    render(<LeftRail initialState={sidebarState()} onNavigate={handleNavigate} />)

    const savedSearchLink = screen.getByText("tenant").closest("a")
    savedSearchLink?.addEventListener("click", (event) => event.preventDefault())
    fireEvent.click(savedSearchLink!)

    expect(handleNavigate).toHaveBeenCalledTimes(1)
  })
})

describe("MobileLeftRailSheet", () => {
  beforeEach(() => {
    pathname = "/statutes"
    searchParams = new URLSearchParams()
  })

  it("exposes the authority sidebar trigger below large viewports", () => {
    render(<MobileLeftRailSheet initialState={sidebarState()} />)

    const trigger = screen.getByRole("button", { name: "Open authority sidebar" })

    expect(trigger).toHaveClass("lg:hidden")
  })
})
