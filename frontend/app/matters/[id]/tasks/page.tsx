import { notFound } from "next/navigation"
import { MatterShell } from "@/components/casebuilder/matter-shell"
import { TasksBoard } from "@/components/casebuilder/tasks-board"
import { getMatterState } from "@/lib/casebuilder/server-api"

export default async function TasksPage({ params }: PageProps<"/matters/[id]/tasks">) {
  const { id } = await params
  const matterState = await getMatterState(id)
  const matter = matterState.data
  if (!matter) notFound()

  return (
    <MatterShell matter={matter} activeSection="tasks" dataState={matterState}>
      <TasksBoard matter={matter} />
    </MatterShell>
  )
}
