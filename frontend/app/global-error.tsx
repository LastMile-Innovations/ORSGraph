"use client"

import { useEffect } from "react"
import { fontVariables } from "./fonts"

export default function GlobalError({
  error,
  unstable_retry,
}: {
  error: Error & { digest?: string }
  unstable_retry: () => void
}) {
  useEffect(() => {
    console.error(error)
  }, [error])

  const message = error.digest
    ? `Unexpected root error. Digest: ${error.digest}`
    : error.message || "Unexpected root error."

  return (
    <html lang="en" className={fontVariables}>
      <body>
        <main
          style={{
            display: "grid",
            minHeight: "100vh",
            placeItems: "center",
            padding: 24,
            fontFamily: "var(--font-inter), system-ui, sans-serif",
          }}
        >
          <section style={{ maxWidth: 520, border: "1px solid #d4d4d8", borderRadius: 6, padding: 24 }}>
            <p style={{ margin: 0, fontSize: 11, letterSpacing: 1.5, textTransform: "uppercase", color: "#dc2626" }}>
              Application error
            </p>
            <h1 style={{ margin: "8px 0 0", fontSize: 20 }}>ORSGraph could not recover the root shell.</h1>
            <p style={{ color: "#52525b", lineHeight: 1.6 }}>
              {message}
            </p>
            <button
              type="button"
              onClick={unstable_retry}
              style={{
                minHeight: 40,
                border: 0,
                borderRadius: 6,
                background: "#18181b",
                color: "white",
                cursor: "pointer",
                padding: "0 14px",
              }}
            >
              Try again
            </button>
          </section>
        </main>
      </body>
    </html>
  )
}
