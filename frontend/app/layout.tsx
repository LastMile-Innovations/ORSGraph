import type { Metadata } from "next"
import { Analytics } from "@vercel/analytics/next"
import { AuthSessionProvider } from "@/components/auth-session-provider"
import { ThemeProvider } from "@/components/theme-provider"
import "./globals.css"

const enableVercelAnalytics = process.env.VERCEL === "1"

export const metadata: Metadata = {
  title: "ORSGraph - Legal Operating Environment",
  description:
    "Source-first legal graph for controlling, persuasive, and official analytical authorities.",
  generator: "v0.app",
}

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode
}>) {
  return (
    <html lang="en" suppressHydrationWarning className="bg-background">
      <body className="font-sans antialiased">
        <ThemeProvider attribute="class" defaultTheme="dark" enableSystem disableTransitionOnChange>
          <AuthSessionProvider>{children}</AuthSessionProvider>
        </ThemeProvider>
        {enableVercelAnalytics && <Analytics />}
      </body>
    </html>
  )
}
