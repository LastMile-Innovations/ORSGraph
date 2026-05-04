import "@testing-library/jest-dom/vitest"
import React from "react"
import { vi } from "vitest"

class ResizeObserverMock {
  observe() {}
  unobserve() {}
  disconnect() {}
}

globalThis.ResizeObserver = globalThis.ResizeObserver ?? ResizeObserverMock

vi.mock("next/link", () => ({
  default: ({
    href,
    children,
    ...props
  }: {
    href: string | { pathname?: string; query?: Record<string, string> }
    children: React.ReactNode
    [key: string]: unknown
  }) =>
    React.createElement(
      "a",
      {
        href: typeof href === "string" ? href : href.pathname ?? "/",
        ...props,
      },
      children,
    ),
  useLinkStatus: () => ({ pending: false }),
}))
