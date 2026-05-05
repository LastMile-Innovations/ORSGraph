import { render, screen, waitFor } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { useState } from "react"
import { beforeEach, describe, expect, it, vi } from "vitest"
import type { SuggestResult } from "@/lib/types"
import { SearchInput } from "./search-input"

const searchSuggest = vi.fn()

vi.mock("@/lib/api", () => ({
  searchSuggest: (...args: unknown[]) => searchSuggest(...args),
}))

function SearchInputHarness({ onSelectSuggestion }: { onSelectSuggestion?: (suggestion: SuggestResult) => void }) {
  const [value, setValue] = useState("")
  return <SearchInput value={value} onChange={setValue} onSelectSuggestion={onSelectSuggestion} />
}

describe("SearchInput", () => {
  beforeEach(() => {
    searchSuggest.mockReset()
    searchSuggest.mockResolvedValue([
      {
        label: "ORS 90.300",
        kind: "statute",
        href: "/statutes/or%3Aors%3A90.300",
        match_type: "exact",
        score: 1,
      },
    ])
  })

  it("closes and suppresses suggestions after selecting an exact authority", async () => {
    const user = userEvent.setup()
    const onSelectSuggestion = vi.fn()

    render(<SearchInputHarness onSelectSuggestion={onSelectSuggestion} />)

    await user.type(screen.getByPlaceholderText(/search statutes/i), "ORS 90")
    const suggestion = await screen.findByRole("button", { name: /ORS 90\.300/i })

    await user.click(suggestion)

    expect(onSelectSuggestion).toHaveBeenCalledWith(expect.objectContaining({ label: "ORS 90.300" }))
    expect(screen.queryByRole("button", { name: /ORS 90\.300/i })).not.toBeInTheDocument()
    await waitFor(() => {
      expect(searchSuggest).toHaveBeenCalledTimes(1)
    })
  })
})
