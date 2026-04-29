import { Shell } from "@/components/orsg/shell"
import { QCConsoleClient } from "@/components/orsg/qc/qc-console-client"

export default function QCPage() {
  return (
    <Shell>
      <QCConsoleClient />
    </Shell>
  )
}
