"use client"

import { useState } from "react"
import { useRouter } from "next/navigation"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogTitle,
} from "@/components/ui/dialog"
import type { DocumentWorkspace as DocumentWorkspaceState, Matter } from "@/lib/casebuilder/types"
import { DocumentWorkspace } from "./document-workspace"

interface DocumentWorkspaceModalProps {
  matter: Matter
  workspace: DocumentWorkspaceState
}

export function DocumentWorkspaceModal({ matter, workspace }: DocumentWorkspaceModalProps) {
  const router = useRouter()
  const [open, setOpen] = useState(true)

  function onOpenChange(nextOpen: boolean) {
    setOpen(nextOpen)
    if (!nextOpen) router.back()
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="h-[92vh] max-h-[92vh] w-[min(96vw,1440px)] max-w-none overflow-hidden rounded-md p-0">
        <DialogTitle className="sr-only">{workspace.document.title}</DialogTitle>
        <DialogDescription className="sr-only">
          Document workspace for {matter.name}
        </DialogDescription>
        <div className="flex h-full min-h-0 overflow-hidden">
          <DocumentWorkspace matter={matter} workspace={workspace} />
        </div>
      </DialogContent>
    </Dialog>
  )
}
