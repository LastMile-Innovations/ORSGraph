import { Shell } from "@/components/orsg/shell"
import { StatuteIndexClient } from "@/components/orsg/statute/statute-index-client"
import { getCachedStatuteIndexState } from "@/lib/authority-server-cache"

type StatuteIndexPageParams = {
  q?: string
  chapter?: string
  status?: string
  limit?: string
  offset?: string
}

type StatuteIndexPageProps = Omit<PageProps<"/statutes">, "searchParams"> & {
  searchParams: Promise<StatuteIndexPageParams>
}

function numberParam(value: string | undefined, fallback: number, max: number) {
  const parsed = Number(value)
  if (!Number.isFinite(parsed) || parsed <= 0) return fallback
  return Math.min(Math.floor(parsed), max)
}

export default async function StatuteIndexPage({
  searchParams,
}: StatuteIndexPageProps) {
  const params = await searchParams
  const q = params.q?.trim() ?? ""
  const chapter = params.chapter?.trim() ?? ""
  const status = params.status?.trim() || "all"
  const limit = numberParam(params.limit, 60, 120)
  const offset = Math.max(0, Number(params.offset || 0) || 0)
  const state = await getCachedStatuteIndexState({ q, chapter, status, limit, offset })

  return (
    <Shell>
      <StatuteIndexClient
        statutes={state.data.items}
        total={state.data.total}
        limit={state.data.limit}
        offset={state.data.offset}
        query={q}
        chapter={chapter}
        status={status}
        dataSource={state.source}
        dataError={state.error}
      />
    </Shell>
  )
}
