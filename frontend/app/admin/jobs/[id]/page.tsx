import { Shell } from "@/components/orsg/shell"
import { AdminJobDetailClient } from "@/components/orsg/admin/admin-job-detail-client"

export const unstable_instant = {
  prefetch: "static",
  unstable_disableValidation: true,
}

export default async function AdminJobPage({ params }: PageProps<"/admin/jobs/[id]">) {
  const { id } = await params

  return (
    <Shell>
      <AdminJobDetailClient jobId={id} />
    </Shell>
  )
}
