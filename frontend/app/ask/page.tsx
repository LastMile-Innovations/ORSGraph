import { Shell } from "@/components/orsg/shell"
import { AskClient } from "@/components/orsg/ask/ask-client"
import { askWithFallback } from "@/lib/api"

export default async function AskPage({
  searchParams,
}: {
  searchParams: Promise<{ q?: string }>
}) {
  const { q } = await searchParams
  const question = q ?? "What Oregon laws define district attorney duties?"
  const answer = await askWithFallback(question)
  return (
    <Shell>
      <AskClient initialQuery={question} initialAnswer={answer} />
    </Shell>
  )
}
