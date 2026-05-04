import { InviteClient } from "./invite-client"

export const unstable_instant = {
  prefetch: "static",
  unstable_disableValidation: true,
}

export default async function InvitePage({ params }: PageProps<"/auth/invite/[token]">) {
  const { token } = await params
  return <InviteClient token={token} />
}
