import { Shell } from "@/components/orsg/shell"
import { AskClient } from "@/components/orsg/ask/ask-client"
import { askAnswer } from "@/lib/mock-data"

export default async function AskPage({
  searchParams,
}: {
  searchParams: Promise<{ q?: string }>
}) {
  const { q } = await searchParams
  return (
    <Shell>
      <AskClient initialQuery={q ?? askAnswer.question} answer={askAnswer} />
    </Shell>
  )
}
