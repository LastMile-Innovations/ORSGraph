import { render, screen } from "@testing-library/react"
import { describe, expect, it } from "vitest"
import { ActionCardGrid } from "./ActionCardGrid"
import type { HomeAction } from "@/lib/types"

describe("ActionCardGrid", () => {
  it("does not advertise Ask as ready while the AI harness is limited beta", () => {
    const actions: HomeAction[] = [
      {
        title: "Ask ORSGraph",
        description: "Ask graph-grounded legal questions.",
        href: "/ask",
        icon: "MessageSquare",
        status: "ready",
        badges: ["QA", "rerank-ready"],
      },
    ]

    render(<ActionCardGrid actions={actions} />)

    expect(screen.queryByText(/^ready$/i)).not.toBeInTheDocument()
    expect(screen.getAllByText(/limited beta/i).length).toBeGreaterThan(0)
    expect(screen.queryByText(/rerank-ready/i)).not.toBeInTheDocument()
  })
})
