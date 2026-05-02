import type { NextAuthOptions } from "next-auth"

type ZitadelProfile = {
  sub: string
  email?: string
  name?: string
  preferred_username?: string
  picture?: string
  [key: string]: unknown
}

const issuer = process.env.ZITADEL_ISSUER?.replace(/\/$/, "")
const scopes = [
  "openid",
  "profile",
  "email",
  "offline_access",
  "urn:iam:org:project:roles",
  "urn:zitadel:iam:org:projects:roles",
]

if (process.env.ZITADEL_PROJECT_ID) {
  scopes.push(`urn:zitadel:iam:org:project:id:${process.env.ZITADEL_PROJECT_ID}:aud`)
}

export const authOptions: NextAuthOptions = {
  session: {
    strategy: "jwt",
  },
  providers: [
    {
      id: "zitadel",
      name: "Zitadel",
      type: "oauth",
      wellKnown: issuer ? `${issuer}/.well-known/openid-configuration` : undefined,
      authorization: {
        params: {
          scope: scopes.join(" "),
        },
      },
      idToken: true,
      checks: ["pkce", "state"],
      clientId: process.env.ZITADEL_CLIENT_ID,
      clientSecret: process.env.ZITADEL_CLIENT_SECRET,
      profile(profile: ZitadelProfile) {
        return {
          id: profile.sub,
          name: profile.name || profile.preferred_username || profile.email || profile.sub,
          email: profile.email,
          image: profile.picture,
        }
      },
    },
  ],
  callbacks: {
    async jwt({ token, account, profile }) {
      if (account) {
        token.accessToken = account.access_token
        token.idToken = account.id_token
      }
      if (profile) {
        const zitadelProfile = profile as ZitadelProfile
        token.sub = zitadelProfile.sub || token.sub
        token.roles = rolesFromProfile(zitadelProfile)
      }
      return token
    },
    async session({ session, token }) {
      session.accessToken = typeof token.accessToken === "string" ? token.accessToken : undefined
      session.idToken = typeof token.idToken === "string" ? token.idToken : undefined
      session.roles = Array.isArray(token.roles) ? token.roles.filter((role): role is string => typeof role === "string") : []
      if (session.user) {
        session.user.id = token.sub || ""
      }
      return session
    },
  },
}

function rolesFromProfile(profile: ZitadelProfile) {
  const roles = new Set<string>()
  for (const [key, value] of Object.entries(profile)) {
    if (key === "roles" || key === "role" || key.endsWith(":roles") || key.includes("project:roles")) {
      collectRoles(value, roles)
    }
  }
  return Array.from(roles).sort()
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
    for (const [key, nested] of Object.entries(value)) {
      if (key.trim()) roles.add(key.trim())
      collectRoles(nested, roles)
    }
  }
}
