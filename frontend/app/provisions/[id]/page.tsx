import { notFound } from "next/navigation"
import { Shell } from "@/components/orsg/shell"
import { ProvisionInspectorClient } from "@/components/orsg/provision/provision-inspector-client"
import { getProvisionById } from "@/lib/mock-data"

export default async function ProvisionPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params
  const decoded = decodeURIComponent(id)
  const data = getProvisionById(decoded)
  if (!data) return notFound()
  return (
    <Shell>
      <ProvisionInspectorClient data={data} />
    </Shell>
  )
}
