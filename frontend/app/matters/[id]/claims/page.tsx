import { notFound } from "next/navigation"
import { MatterShell } from "@/components/casebuilder/matter-shell"
import { ClaimsBuilder } from "@/components/casebuilder/claims-builder"
import { getMatterById } from "@/lib/casebuilder/mock-matters"

interface PageProps {
  params: Promise<{ id: string }>
}

export default async function ClaimsPage({ params }: PageProps) {
  const { id } = await params
  const matter = getMatterById(id)
  if (!matter) notFound()
  return (
    <MatterShell matter={matter} activeSection="claims">
      <ClaimsBuilder matter={matter} />
    </MatterShell>
  )
}
