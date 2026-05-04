import { Shell } from "@/components/orsg/shell"
import { QCConsoleClient } from "@/components/orsg/admin/qc-console-client"

export default function AdminQCPage() {
  return (
    <Shell>
      <QCConsoleClient />
    </Shell>
  )
}
