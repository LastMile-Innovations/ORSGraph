"use client"

import { useState } from "react"
import { useRouter } from "next/navigation"
import { Trash2 } from "lucide-react"
import {
  AlertDialog,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { deleteMatter } from "@/lib/casebuilder/api"
import { casebuilderHomeHref } from "@/lib/casebuilder/routes"
import { cn } from "@/lib/utils"
import type { MatterSummary } from "@/lib/casebuilder/types"

interface DeleteMatterButtonProps {
  matter: Pick<MatterSummary, "matter_id" | "name">
  className?: string
  compact?: boolean
}

export function DeleteMatterButton({
  matter,
  className,
  compact = false,
}: DeleteMatterButtonProps) {
  const router = useRouter()
  const [open, setOpen] = useState(false)
  const [confirmation, setConfirmation] = useState("")
  const [deleting, setDeleting] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const confirmationValue = matter.name.trim()
  const confirmed = confirmation.trim() === confirmationValue

  function reset() {
    setConfirmation("")
    setError(null)
    setDeleting(false)
  }

  async function handleDelete() {
    if (!confirmed || deleting) return

    setDeleting(true)
    setError(null)
    const result = await deleteMatter(matter.matter_id)
    if (!result.data?.deleted) {
      setError(result.error ?? "Matter delete failed.")
      setDeleting(false)
      return
    }

    window.localStorage.removeItem(`casebuilder:ask:${matter.matter_id}`)
    router.replace(casebuilderHomeHref())
    router.refresh()
  }

  return (
    <AlertDialog
      open={open}
      onOpenChange={(nextOpen) => {
        if (deleting) return
        setOpen(nextOpen)
        if (!nextOpen) reset()
      }}
    >
      <AlertDialogTrigger asChild>
        <Button
          type="button"
          variant="destructive"
          size={compact ? "sm" : "default"}
          className={cn("font-mono text-xs uppercase tracking-wider", className)}
        >
          <Trash2 className="h-3.5 w-3.5" />
          {compact ? "Delete" : "Delete matter"}
        </Button>
      </AlertDialogTrigger>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Delete matter permanently?</AlertDialogTitle>
          <AlertDialogDescription>
            This permanently deletes the matter, its CaseBuilder graph, uploaded files,
            derived artifacts, transcripts, drafts, timeline work, and local matter chat.
          </AlertDialogDescription>
        </AlertDialogHeader>

        <div className="space-y-2">
          <label htmlFor="delete-matter-confirmation" className="text-sm font-medium text-foreground">
            Type <span className="font-mono">{confirmationValue}</span> to confirm.
          </label>
          <Input
            id="delete-matter-confirmation"
            value={confirmation}
            onChange={(event) => setConfirmation(event.target.value)}
            disabled={deleting}
            aria-label={`Type ${confirmationValue} to confirm`}
          />
          {error ? (
            <p role="alert" className="rounded border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive">
              {error}
            </p>
          ) : null}
        </div>

        <AlertDialogFooter>
          <AlertDialogCancel disabled={deleting}>Cancel</AlertDialogCancel>
          <Button
            type="button"
            variant="destructive"
            onClick={handleDelete}
            disabled={!confirmed || deleting}
          >
            <Trash2 className="h-4 w-4" />
            {deleting ? "Deleting..." : "Delete permanently"}
          </Button>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  )
}
