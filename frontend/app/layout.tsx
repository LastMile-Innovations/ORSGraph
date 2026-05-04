import type { Metadata, Viewport } from "next"
import { Analytics } from "@vercel/analytics/next"
import { AuthSessionProvider } from "@/components/auth-session-provider"
import { CaseBuilderUploadProvider } from "@/components/casebuilder/upload-provider"
import { ThemeProvider } from "@/components/theme-provider"
import { WebVitals } from "@/components/web-vitals"
import { siteOrigin } from "./metadata"
import { fontVariables } from "./fonts"
import "./globals.css"

const enableVercelAnalytics = process.env.VERCEL === "1"
const enableWebVitals = Boolean(process.env.NEXT_PUBLIC_WEB_VITALS_ENDPOINT)

export const metadata: Metadata = {
  metadataBase: new URL(siteOrigin()),
  title: {
    default: "ORSGraph - Legal Operating Environment",
    template: "%s | ORSGraph",
  },
  description:
    "Source-first legal graph for controlling, persuasive, and official analytical authorities.",
  applicationName: "ORSGraph",
  generator: "Next.js",
  alternates: {
    canonical: "/",
  },
  openGraph: {
    title: "ORSGraph - Legal Operating Environment",
    description:
      "Source-first legal graph for controlling, persuasive, and official analytical authorities.",
    url: "/",
    siteName: "ORSGraph",
    type: "website",
  },
  twitter: {
    card: "summary",
    title: "ORSGraph - Legal Operating Environment",
    description:
      "Source-first legal graph for controlling, persuasive, and official analytical authorities.",
  },
}

export const viewport: Viewport = {
  themeColor: [
    { media: "(prefers-color-scheme: light)", color: "#fbfaf7" },
    { media: "(prefers-color-scheme: dark)", color: "#060c10" },
  ],
  colorScheme: "dark light",
}

export default function RootLayout({
  children,
}: Readonly<LayoutProps<"/">>) {
  return (
    <html lang="en" suppressHydrationWarning className={`${fontVariables} bg-background`}>
      <body className="font-sans antialiased">
        <ThemeProvider attribute="class" defaultTheme="dark" enableSystem disableTransitionOnChange>
          <AuthSessionProvider>
            <CaseBuilderUploadProvider>{children}</CaseBuilderUploadProvider>
          </AuthSessionProvider>
        </ThemeProvider>
        {enableWebVitals && <WebVitals />}
        {enableVercelAnalytics && <Analytics />}
      </body>
    </html>
  )
}
