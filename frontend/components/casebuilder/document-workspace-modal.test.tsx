import { fireEvent, render, screen } from "@testing-library/react"
import { describe, expect, it, vi } from "vitest"
import type { DocumentWorkspace as DocumentWorkspaceState, Matter } from "@/lib/casebuilder/types"
import { DocumentWorkspaceModal } from "./document-workspace-modal"

const router = {
  back: vi.fn(),
}

vi.mock("next/navigation", () => ({
  useRouter: () => router,
}))

vi.mock("./document-workspace", () => ({
  DocumentWorkspace: ({ workspace }: { workspace: DocumentWorkspaceState }) => (
    <div data-testid="document-workspace">{workspace.document.title}</div>
  ),
}))

describe("DocumentWorkspaceModal", () => {
  it("renders the document workspace and closes through browser history", () => {
    router.back.mockReset()

    render(
      <DocumentWorkspaceModal
        matter={{ name: "Smith v. ABC" } as Matter}
        workspace={{ document: { title: "Lease ledger.pdf" } } as DocumentWorkspaceState}
      />,
    )

    expect(screen.getByTestId("document-workspace")).toHaveTextContent("Lease ledger.pdf")

    fireEvent.click(screen.getByRole("button", { name: "Close" }))

    expect(router.back).toHaveBeenCalledTimes(1)
  })
})
