import "@testing-library/jest-dom/vitest"
import React from "react"
import { vi } from "vitest"

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
}))
