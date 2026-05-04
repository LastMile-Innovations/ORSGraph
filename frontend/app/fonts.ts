import { Inter, JetBrains_Mono } from "next/font/google"

export const inter = Inter({
  display: "swap",
  fallback: ["system-ui", "Arial", "sans-serif"],
  subsets: ["latin"],
  variable: "--font-inter",
})

export const jetBrainsMono = JetBrains_Mono({
  display: "swap",
  fallback: ["ui-monospace", "SFMono-Regular", "Menlo", "Monaco", "Consolas", "Liberation Mono", "monospace"],
  subsets: ["latin"],
  variable: "--font-jetbrains-mono",
})

export const fontVariables = `${inter.variable} ${jetBrainsMono.variable}`
