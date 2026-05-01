"use client"

import * as React from "react"

type Theme = "light" | "dark" | "system"
type ResolvedTheme = "light" | "dark"

interface ThemeContextValue {
  theme: Theme
  resolvedTheme: ResolvedTheme
  setTheme: (theme: Theme) => void
}

interface ThemeProviderProps {
  children: React.ReactNode
  attribute?: "class" | "data-theme"
  defaultTheme?: Theme
  enableSystem?: boolean
  disableTransitionOnChange?: boolean
  storageKey?: string
}

const ThemeContext = React.createContext<ThemeContextValue | undefined>(undefined)

function getSystemTheme(): ResolvedTheme {
  if (typeof window === "undefined") return "dark"
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light"
}

function resolveTheme(theme: Theme, enableSystem: boolean): ResolvedTheme {
  if (theme === "system" && enableSystem) return getSystemTheme()
  return theme === "light" ? "light" : "dark"
}

function applyTheme(theme: ResolvedTheme, attribute: "class" | "data-theme") {
  const root = document.documentElement
  if (attribute === "class") {
    root.classList.remove("light", "dark")
    root.classList.add(theme)
  } else {
    root.setAttribute(attribute, theme)
  }
  root.style.colorScheme = theme
}

export function ThemeProvider({
  children,
  attribute = "class",
  defaultTheme = "dark",
  enableSystem = true,
  storageKey = "theme",
}: ThemeProviderProps) {
  const [theme, setThemeState] = React.useState<Theme>(defaultTheme)
  const [resolvedTheme, setResolvedTheme] = React.useState<ResolvedTheme>(() =>
    resolveTheme(defaultTheme, enableSystem),
  )

  React.useEffect(() => {
    const stored = window.localStorage.getItem(storageKey) as Theme | null
    if (stored === "light" || stored === "dark" || stored === "system") {
      setThemeState(stored)
      setResolvedTheme(resolveTheme(stored, enableSystem))
    }
  }, [enableSystem, storageKey])

  React.useEffect(() => {
    const next = resolveTheme(theme, enableSystem)
    setResolvedTheme(next)
    applyTheme(next, attribute)
  }, [attribute, enableSystem, theme])

  React.useEffect(() => {
    if (!enableSystem || theme !== "system") return undefined
    const media = window.matchMedia("(prefers-color-scheme: dark)")
    const handleChange = () => {
      const next = getSystemTheme()
      setResolvedTheme(next)
      applyTheme(next, attribute)
    }
    media.addEventListener("change", handleChange)
    return () => media.removeEventListener("change", handleChange)
  }, [attribute, enableSystem, theme])

  const setTheme = React.useCallback(
    (nextTheme: Theme) => {
      setThemeState(nextTheme)
      window.localStorage.setItem(storageKey, nextTheme)
    },
    [storageKey],
  )

  const value = React.useMemo(
    () => ({ theme, resolvedTheme, setTheme }),
    [resolvedTheme, setTheme, theme],
  )

  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>
}

export function useTheme() {
  const context = React.useContext(ThemeContext)
  if (!context) {
    return {
      theme: "dark" as Theme,
      resolvedTheme: "dark" as ResolvedTheme,
      setTheme: () => {},
    }
  }
  return context
}
