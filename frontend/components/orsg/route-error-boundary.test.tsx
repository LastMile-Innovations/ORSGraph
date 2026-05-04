import { fireEvent, render, screen } from "@testing-library/react"
import { describe, expect, it, vi } from "vitest"
import { RouteErrorBoundary } from "./route-error-boundary"

describe("RouteErrorBoundary", () => {
  it("uses unstable_retry for recovery and shows the digest without server details", () => {
    const unstableRetry = vi.fn()
    const error = new Error("database password leaked")
    Object.assign(error, { digest: "abc123" })

    render(
      <RouteErrorBoundary
        error={error}
        unstable_retry={unstableRetry}
        title="Route failed"
        homeHref="/dashboard"
        homeLabel="Dashboard"
      />,
    )

    expect(screen.getByText("Route failed")).toBeInTheDocument()
    expect(screen.getByText("The route could not render. Error digest: abc123")).toBeInTheDocument()
    expect(screen.queryByText(/database password leaked/)).not.toBeInTheDocument()

    fireEvent.click(screen.getByRole("button", { name: "Try again" }))
    expect(unstableRetry).toHaveBeenCalledTimes(1)
  })
})
