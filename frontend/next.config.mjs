/** @type {import('next').NextConfig} */
const nextConfig = {
  cacheComponents: true,
  cacheLife: {
    authorityShell: {
      stale: 300,
      revalidate: 60,
      expire: 3600,
    },
    authorityDetail: {
      stale: 300,
      revalidate: 3600,
      expire: 86400,
    },
  },
  experimental: {
    optimizePackageImports: [
      "@radix-ui/react-accordion",
      "@radix-ui/react-dialog",
      "@radix-ui/react-dropdown-menu",
      "@radix-ui/react-select",
      "@radix-ui/react-tabs",
      "@radix-ui/react-tooltip",
    ],
  },
  images: {
    formats: ["image/webp"],
    localPatterns: [
      {
        pathname: "/marketing/**",
        search: "",
      },
    ],
    qualities: [75, 90],
  },
}

export default nextConfig
