import { describe, expect, it } from "vitest"
import { cn } from "./utils"

describe("cn", () => {
  it("merges conditional class names", () => {
    expect(cn("rounded-md", false && "hidden", "border")).toBe("rounded-md border")
  })

  it("lets later Tailwind utilities win", () => {
    expect(cn("p-2 text-sm", "p-4", "text-lg")).toBe("p-4 text-lg")
  })
})
