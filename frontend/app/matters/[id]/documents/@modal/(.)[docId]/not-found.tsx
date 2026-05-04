"use client"

import { useState } from "react"
import { useRouter } from "next/navigation"
import { FileQuestion } from "lucide-react"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogTitle,
} from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"

export default function DocumentModalNotFound() {
  const router = useRouter()
  const [open, setOpen] = useState(true)

  function closeModal() {
    setOpen(false)
    router.back()
  }

  return (
    <Dialog open={open} onOpenChange={closeModal}>
      <DialogContent className="w-[min(92vw,440px)] rounded-md">
        <div className="flex items-start gap-3">
          <FileQuestion className="mt-0.5 h-5 w-5 flex-none text-primary" />
          <div>
            <DialogTitle>Document not found</DialogTitle>
            <DialogDescription className="mt-2">
              That document is not available in the current matter workspace.
            </DialogDescription>
            <Button type="button" size="sm" className="mt-4" onClick={closeModal}>
              Close
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  )
}
