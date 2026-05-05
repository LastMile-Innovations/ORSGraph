import { AskClient } from "@/components/orsg/ask/ask-client"
import { askWithFallbackState } from "@/lib/api"

export const unstable_instant = {
  prefetch: "static",
  unstable_disableValidation: true,
}

type AskPageProps = Omit<PageProps<"/ask">, "searchParams"> & {
  searchParams: Promise<{ q?: string }>
}

export default async function AskPage({
  searchParams,
}: AskPageProps) {
  const { q } = await searchParams
  const question = q?.trim() ?? ""
  const answerState = question ? await askWithFallbackState(question) : undefined
  return (
    <AskClient
      initialQuery={question}
      initialAnswer={answerState?.data ?? null}
      initialDataSource={answerState?.source}
      initialDataError={answerState?.error}
    />
  )
}
