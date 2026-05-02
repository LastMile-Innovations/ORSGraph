import type { NextAuthOptions } from "next-auth"
import ZitadelProvider, { type ZitadelProfile } from "next-auth/providers/zitadel"
import { orsBackendApiBaseUrl } from "./ors-api-url"

type AccessStatus = "active" | "pending" | "blocked" | "unknown"

export type AuthMeResponse = {
  access_status?: string
  roles?: string[]
  is_admin?: boolean
}

const issuer = process.env.ZITADEL_ISSUER?.replace(/\/$/, "")
const scopes = [
  "openid",
  "profile",
  "email",
  "offline_access",
]

if (process.env.ZITADEL_PROJECT_ID) {
  scopes.push(`urn:zitadel:iam:org:project:id:${process.env.ZITADEL_PROJECT_ID}:aud`)
  scopes.push("urn:zitadel:iam:org:projects:roles")
}
scopes.push("urn:iam:org:project:roles")

export const authOptions: NextAuthOptions = {
  pages: {
    signIn: "/auth/signin",
    error: "/auth/error",
  },
  session: {
    strategy: "jwt",
  },
  providers: [
    ZitadelProvider({
      issuer: issuer ?? "",
      authorization: {
        params: {
          scope: Array.from(new Set(scopes)).join(" "),
        },
      },
      clientId: process.env.ZITADEL_CLIENT_ID ?? "",
      clientSecret: process.env.ZITADEL_CLIENT_SECRET ?? "",
      profile(profile: ZitadelProfile) {
        return {
          id: profile.sub,
          name: profile.name || profile.preferred_username || profile.email || profile.sub,
          email: profile.email,
          image: profile.picture,
        }
      },
    }),
  ],
  callbacks: {
    async jwt({ token, account, profile, trigger }) {
      const roles = new Set(Array.isArray(token.roles) ? token.roles.filter(isString) : [])
      if (account) {
        token.accessToken = account.access_token
        token.idToken = account.id_token
        token.accessCheckedAt = 0
        addRolesFromClaims(decodeJwtPayload(account.access_token), roles)
        addRolesFromClaims(decodeJwtPayload(account.id_token), roles)
      }
      if (profile) {
        const zitadelProfile = profile as ZitadelProfile
        token.sub = zitadelProfile.sub || token.sub
        addRolesFromClaims(zitadelProfile, roles)
      }
      token.roles = Array.from(roles).sort()
      if (shouldRefreshAccess(token.accessCheckedAt, trigger) && typeof token.accessToken === "string") {
        const access = await fetchAccessState(token.accessToken)
        token.accessStatus = access.accessStatus
        token.accessCheckedAt = Date.now()
        token.isAdmin = access.isAdmin
        token.roles = access.roles.length > 0 ? access.roles : token.roles
      }
      return token
    },
    async session({ session, token }) {
      session.accessToken = typeof token.accessToken === "string" ? token.accessToken : undefined
      session.idToken = typeof token.idToken === "string" ? token.idToken : undefined
      session.accessStatus = normalizeAccessStatus(token.accessStatus)
      session.accessCheckedAt = typeof token.accessCheckedAt === "number" ? token.accessCheckedAt : undefined
      session.isAdmin = Boolean(token.isAdmin)
      session.roles = Array.isArray(token.roles) ? token.roles.filter((role): role is string => typeof role === "string") : []
      if (session.user) {
        session.user.id = token.sub || ""
      }
      return session
    },
  },
}

function shouldRefreshAccess(checkedAt: unknown, trigger?: string) {
  if (trigger === "update") return true
  if (typeof checkedAt !== "number" || checkedAt <= 0) return true
  return Date.now() - checkedAt > 5 * 60 * 1000
}

async function fetchAccessState(accessToken: string): Promise<{
  accessStatus: AccessStatus
  roles: string[]
  isAdmin: boolean
}> {
  try {
    const response = await fetch(`${orsBackendApiBaseUrl()}/auth/me`, {
      cache: "no-store",
      headers: {
        Authorization: `Bearer ${accessToken}`,
      },
    })
    if (!response.ok) {
      return { accessStatus: response.status === 403 ? "pending" : "unknown", roles: [], isAdmin: false }
    }
    return accessStateFromAuthMe((await response.json()) as AuthMeResponse)
  } catch {
    return { accessStatus: "unknown", roles: [], isAdmin: false }
  }
}

export function accessStateFromAuthMe(body: AuthMeResponse) {
  const isAdmin = Boolean(body.is_admin)
  return {
    accessStatus: isAdmin ? "active" : normalizeAccessStatus(body.access_status),
    roles: Array.isArray(body.roles) ? body.roles.filter((role): role is string => typeof role === "string") : [],
    isAdmin,
  }
}

function normalizeAccessStatus(value: unknown): AccessStatus {
  return value === "active" || value === "pending" || value === "blocked" ? value : "unknown"
}

export function rolesFromZitadelClaims(claims: Record<string, unknown> | null | undefined) {
  const roles = new Set<string>()
  addRolesFromClaims(claims, roles)
  return Array.from(roles).sort()
}

function addRolesFromClaims(claims: Record<string, unknown> | null | undefined, roles: Set<string>) {
  if (!claims) return
  for (const [key, value] of Object.entries(claims)) {
    if (isRoleClaimKey(key)) collectRoles(value, roles)
  }
}

function isRoleClaimKey(key: string) {
  return (
    key === "roles" ||
    key === "role" ||
    key === "urn:iam:org:project:roles" ||
    key === "urn:zitadel:iam:org:project:roles" ||
    /^urn:zitadel:iam:org:project:[^:]+:roles$/.test(key)
  )
}

function collectRoles(value: unknown, roles: Set<string>) {
  if (typeof value === "string" && value.trim()) {
    roles.add(value.trim())
    return
  }
  if (Array.isArray(value)) {
    value.forEach((item) => collectRoles(item, roles))
    return
  }
  if (value && typeof value === "object") {
    for (const key of Object.keys(value)) {
      if (key.trim()) roles.add(key.trim())
    }
  }
}

function decodeJwtPayload(token: unknown): Record<string, unknown> | null {
  if (typeof token !== "string") return null
  const [, payload] = token.split(".")
  if (!payload) return null
  try {
    const decoded = Buffer.from(payload, "base64url").toString("utf8")
    const parsed = JSON.parse(decoded) as unknown
    return parsed && typeof parsed === "object" && !Array.isArray(parsed) ? (parsed as Record<string, unknown>) : null
  } catch {
    return null
  }
}

function isString(value: unknown): value is string {
  return typeof value === "string"
}
