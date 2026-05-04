"use client"

import { useEffect, useRef, useState } from "react"
import { FolderUp, Upload } from "lucide-react"
import { getMatterSettingsState } from "@/lib/casebuilder/api"
import type { CaseBuilderEffectiveSettings } from "@/lib/casebuilder/types"
import { filesToUploadCandidates } from "@/lib/casebuilder/upload-folders"
import { uploadOptionsFromEffectiveSettings, useCaseBuilderUploads } from "./upload-provider"

export function MatterUploadControls({ matterId }: { matterId: string }) {
  const fileInputRef = useRef<HTMLInputElement>(null)
  const folderInputRef = useRef<HTMLInputElement>(null)
  const { enqueueMatterUploads } = useCaseBuilderUploads()
  const [message, setMessage] = useState<string | null>(null)
  const [settings, setSettings] = useState<CaseBuilderEffectiveSettings | null>(null)

  useEffect(() => {
    let cancelled = false
    async function loadSettings() {
      const result = await getMatterSettingsState(matterId)
      if (!cancelled) setSettings(result.data?.effective ?? null)
    }
    void loadSettings()
    return () => {
      cancelled = true
    }
  }, [matterId])

  function startUpload(files: FileList | null) {
    if (!files?.length) return
    const candidates = filesToUploadCandidates(files)
    const batchId = enqueueMatterUploads(matterId, candidates, {
      label: candidates.some((candidate) => candidate.relativePath.includes("/")) ? "Folder upload" : "File upload",
      ...uploadOptionsFromEffectiveSettings(settings),
    })
    if (batchId) setMessage("Upload started.")
  }

  return (
    <div className="flex items-center justify-between gap-3 border-b border-border bg-background px-3 py-2">
      <input
        ref={fileInputRef}
        type="file"
        multiple
        hidden
        onChange={(event) => {
          startUpload(event.currentTarget.files)
          event.currentTarget.value = ""
        }}
      />
      <input
        ref={folderInputRef}
        type="file"
        multiple
        hidden
        {...({ webkitdirectory: "", directory: "" } as Record<string, string>)}
        onChange={(event) => {
          startUpload(event.currentTarget.files)
          event.currentTarget.value = ""
        }}
      />
      <div className="min-w-0 truncate font-mono text-[10px] uppercase tracking-widest text-muted-foreground">
        {message ?? "CaseBuilder uploads"}
      </div>
      <div className="flex shrink-0 items-center gap-2">
        <button
          type="button"
          onClick={() => folderInputRef.current?.click()}
          className="inline-flex items-center gap-1.5 rounded border border-border px-2.5 py-1.5 font-mono text-[10px] uppercase tracking-wider text-muted-foreground hover:bg-muted hover:text-foreground"
        >
          <FolderUp className="h-3.5 w-3.5" />
          folder
        </button>
        <button
          type="button"
          onClick={() => fileInputRef.current?.click()}
          className="inline-flex items-center gap-1.5 rounded bg-primary px-2.5 py-1.5 font-mono text-[10px] uppercase tracking-wider text-primary-foreground hover:bg-primary/90"
        >
          <Upload className="h-3.5 w-3.5" />
          files
        </button>
      </div>
    </div>
  )
}
