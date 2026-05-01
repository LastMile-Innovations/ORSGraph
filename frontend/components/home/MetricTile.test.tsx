import { render, screen } from "@testing-library/react"
import { describe, expect, it } from "vitest"
import { MetricTile } from "./MetricTile"

describe("MetricTile", () => {
  it("formats numeric values and helper text", () => {
    render(<MetricTile label="Sections" value={1234} state="ok" helper="indexed" />)

    expect(screen.getByText("Sections")).toBeInTheDocument()
    expect(screen.getByText("1,234")).toBeInTheDocument()
    expect(screen.getByText("indexed")).toHaveClass("text-success")
  })

  it("renders navigable metric cards when href is provided", () => {
    render(<MetricTile label="Statutes" value="ready" state="unknown" href="/statutes" />)

    expect(screen.getByRole("link", { name: /statutes ready/i })).toHaveAttribute("href", "/statutes")
  })

  it("uses state-specific helper colors", () => {
    const { rerender } = render(<MetricTile label="Edges" value={0} state="warning" helper="needs sync" />)

    expect(screen.getByText("needs sync")).toHaveClass("text-warning")

    rerender(<MetricTile label="Edges" value={0} state="error" helper="failed" />)
    expect(screen.getByText("failed")).toHaveClass("text-destructive")

    rerender(<MetricTile label="Edges" value={0} state="unknown" helper="not checked" />)
    expect(screen.getByText("not checked")).toHaveClass("text-muted-foreground")
  })
})
