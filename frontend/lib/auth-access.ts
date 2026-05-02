export type AccessStatus = "active" | "pending" | "blocked" | "unknown"

export interface AccessRequestInput {
  email: string
  situation_type?: string
  deadline_urgency?: string
  jurisdiction?: string
  note?: string
}

export interface AccessRequestResponse {
  ok: boolean
  status: string
  message: string
}

export interface InviteLookupResponse {
  found: boolean
  status: string
  email?: string | null
  situation_type?: string | null
  deadline_urgency?: string | null
  jurisdiction?: string | null
  expires_at?: string | null
  accepted_at?: string | null
}

export interface AuthMeResponse {
  authenticated: boolean
  access_status: AccessStatus
  subject?: string | null
  email?: string | null
  name?: string | null
  roles: string[]
  is_admin: boolean
  profile?: {
    subject: string
    email?: string | null
    name?: string | null
    status: AccessStatus
    situation_type?: string | null
    deadline_urgency?: string | null
    jurisdiction?: string | null
    first_matter_id?: string | null
    onboarding_completed_at?: string | null
  } | null
}

export interface BetaInvite {
  invite_id: string
  email?: string | null
  status: string
  roles: string[]
  situation_type?: string | null
  deadline_urgency?: string | null
  jurisdiction?: string | null
  created_at: string
  updated_at: string
  expires_at: string
  accepted_at?: string | null
  revoked_at?: string | null
}

export interface InviteRequest {
  request_id: string
  email: string
  status: string
  situation_type?: string | null
  deadline_urgency?: string | null
  jurisdiction?: string | null
  note?: string | null
  fulfilled_invite_id?: string | null
  created_at: string
  updated_at: string
}

export interface CreateInviteInput {
  email?: string
  roles?: string[]
  situation_type?: string
  deadline_urgency?: string
  jurisdiction?: string
  expires_in_days?: number
}

export interface CreateInviteResponse {
  invite: BetaInvite
  token: string
  invite_url_path: string
}

export async function requestAccess(input: AccessRequestInput): Promise<AccessRequestResponse> {
  return fetchJson("/api/ors/auth/access-request", {
    method: "POST",
    body: JSON.stringify(input),
  })
}

export async function lookupInvite(token: string): Promise<InviteLookupResponse> {
  return fetchJson(`/api/ors/auth/invites/${encodeURIComponent(token)}`)
}

export async function acceptInvite(token: string): Promise<AuthMeResponse> {
  return fetchJson(`/api/ors/auth/invites/${encodeURIComponent(token)}/accept`, {
    method: "POST",
  })
}

export async function listAccessRequests(): Promise<InviteRequest[]> {
  return fetchJson("/api/ors/admin/auth/access-requests")
}

export async function listInvites(): Promise<BetaInvite[]> {
  return fetchJson("/api/ors/admin/auth/invites")
}

export async function createInvite(input: CreateInviteInput): Promise<CreateInviteResponse> {
  return fetchJson("/api/ors/admin/auth/invites", {
    method: "POST",
    body: JSON.stringify(input),
  })
}

export async function revokeInvite(inviteId: string): Promise<BetaInvite> {
  return fetchJson(`/api/ors/admin/auth/invites/${encodeURIComponent(inviteId)}/revoke`, {
    method: "POST",
  })
}

async function fetchJson<T>(url: string, options: RequestInit = {}): Promise<T> {
  const headers = new Headers(options.headers)
  if (!headers.has("Content-Type") && typeof options.body === "string") {
    headers.set("Content-Type", "application/json")
  }
  const response = await fetch(url, {
    cache: "no-store",
    ...options,
    headers,
  })
  const body = await response.json().catch(() => ({}))
  if (!response.ok) {
    throw new Error(typeof body.error === "string" ? body.error : `Request failed: ${response.status}`)
  }
  return body as T
}
