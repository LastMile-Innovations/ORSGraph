"use client"

import { useReportWebVitals } from "next/web-vitals"

type ReportWebVitalsCallback = Parameters<typeof useReportWebVitals>[0]

const endpoint = process.env.NEXT_PUBLIC_WEB_VITALS_ENDPOINT

const reportWebVitals: ReportWebVitalsCallback = (metric) => {
  if (!endpoint) return

  const body = JSON.stringify({
    id: metric.id,
    name: metric.name,
    delta: metric.delta,
    navigationType: metric.navigationType,
    rating: metric.rating,
    value: metric.value,
    path: window.location.pathname,
  })

  if (navigator.sendBeacon) {
    navigator.sendBeacon(endpoint, body)
    return
  }

  void fetch(endpoint, {
    body,
    cache: "no-store",
    method: "POST",
    keepalive: true,
    headers: {
      "content-type": "application/json",
    },
  })
}

export function WebVitals() {
  useReportWebVitals(reportWebVitals)
  return null
}
