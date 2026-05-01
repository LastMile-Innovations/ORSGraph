import { Shell } from "@/components/orsg/shell"
import { AskClient } from "@/components/orsg/ask/ask-client"
import { askWithFallbackState } from "@/lib/api"

export default async function AskPage({
  searchParams,
}: {
  searchParams: Promise<{ q?: string }>
}) {
  const { q } = await searchParams
  const question = q ?? "What Oregon laws define district attorney duties?"
  const answerState = await askWithFallbackState(question)
  return (
    <Shell>
      <AskClient
        initialQuery={question}
        initialAnswer={answerState.data}
        initialDataSource={answerState.source}
        initialDataError={answerState.error}
      />
    </Shell>
  )
}
