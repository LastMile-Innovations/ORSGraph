"use client"

import { useState } from "react"
import { BookOpen, PanelLeftOpen } from "lucide-react"
import { Button } from "@/components/ui/button"
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from "@/components/ui/sheet"
import type { DataState } from "@/lib/data-state"
import type { SidebarData } from "@/lib/api"
import { LeftRail } from "./left-rail"

interface MobileLeftRailSheetProps {
  initialState: DataState<SidebarData | null> | null
}

export function MobileLeftRailSheet({ initialState }: MobileLeftRailSheetProps) {
  const [open, setOpen] = useState(false)

  return (
    <Sheet open={open} onOpenChange={setOpen}>
      <SheetTrigger asChild>
        <Button variant="ghost" size="icon-sm" className="lg:hidden" aria-label="Open authority sidebar">
          <PanelLeftOpen className="h-4 w-4" />
        </Button>
      </SheetTrigger>
      <SheetContent
        side="left"
        className="w-[18rem] max-w-[88vw] gap-0 border-sidebar-border bg-sidebar p-0 text-sidebar-foreground"
      >
        <SheetHeader className="border-b border-sidebar-border pr-12">
          <SheetTitle className="flex items-center gap-2 text-sm">
            <BookOpen className="h-4 w-4" />
            Authority sidebar
          </SheetTitle>
          <SheetDescription className="font-mono text-[10px] uppercase tracking-widest">
            Oregon Revised Statutes / 2025
          </SheetDescription>
        </SheetHeader>
        <div className="min-h-0 flex-1 overflow-hidden">
          <LeftRail
            initialState={initialState}
            className="w-full border-r-0"
            onNavigate={() => setOpen(false)}
          />
        </div>
      </SheetContent>
    </Sheet>
  )
}
