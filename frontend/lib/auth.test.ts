import { describe, expect, it } from "vitest"
import { accessStateFromAuthMe, rolesFromZitadelClaims } from "./auth"

describe("Zitadel auth claim helpers", () => {
  it("extracts latest project-scoped role keys without leaking org ids or domains", () => {
    expect(
      rolesFromZitadelClaims({
        "urn:zitadel:iam:org:project:123456:roles": {
          orsgraph_admin: {
            "371183997394471278": "lastmile.example",
          },
          reviewer: {
            "371183997394471278": "lastmile.example",
          },
        },
      }),
    ).toEqual(["orsgraph_admin", "reviewer"])
  })

  it("keeps backwards-compatible role claim shapes", () => {
    expect(
      rolesFromZitadelClaims({
        role: "owner",
        roles: ["editor"],
        "urn:iam:org:project:roles": {
          legacy_admin: {
            "371183997394471278": "lastmile.example",
          },
        },
        "urn:zitadel:iam:org:project:roles": {
          project_admin: {
            "371183997394471278": "lastmile.example",
          },
        },
      }),
    ).toEqual(["editor", "legacy_admin", "owner", "project_admin"])
  })

  it("treats backend-confirmed admins as active even before user access approval", () => {
    expect(
      accessStateFromAuthMe({
        access_status: "pending",
        roles: ["orsgraph_admin"],
        is_admin: true,
      }),
    ).toEqual({
      accessStatus: "active",
      roles: ["orsgraph_admin"],
      isAdmin: true,
    })
  })
})
