import { Shell } from "@/components/orsg/shell"
import { AdminJobDetailClient } from "@/components/orsg/admin/admin-job-detail-client"

interface AdminJobPageProps {
  params: Promise<{ id: string }>
}

export default async function AdminJobPage({ params }: AdminJobPageProps) {
  const { id } = await params

  return (
    <Shell>
      <AdminJobDetailClient jobId={id} />
    </Shell>
  )
}
