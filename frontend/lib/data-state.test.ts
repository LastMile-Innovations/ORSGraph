import { describe, expect, it } from "vitest"
import { classifyFallbackSource, dataErrorMessage, isFallbackSource } from "./data-state"

describe("data-state helpers", () => {
  it("normalizes unknown thrown values into user-facing messages", () => {
    expect(dataErrorMessage(new Error("fetch failed"))).toBe("fetch failed")
    expect(dataErrorMessage("plain failure")).toBe("plain failure")
    expect(dataErrorMessage({})).toBe("Unknown error")
  })

  it("classifies network-style failures as offline", () => {
    for (const message of ["fetch failed", "ECONNREFUSED", "ENOTFOUND", "network down", "timed out", "aborted"]) {
      expect(classifyFallbackSource(new Error(message))).toBe("offline")
    }
  })

  it("treats non-network failures as mock fallback and only live as non-fallback", () => {
    expect(classifyFallbackSource(new Error("schema mismatch"))).toBe("mock")
    expect(isFallbackSource("live")).toBe(false)
    expect(isFallbackSource("mock")).toBe(true)
    expect(isFallbackSource(undefined)).toBe(false)
  })
})
