import Link from "next/link"
import { HomeAction } from "@/lib/types"
import { Search, MessageSquare, BookOpen, Network, ShieldCheck, Activity } from "lucide-react"
import { cn } from "@/lib/utils"

const IconMap: Record<string, any> = {
  Search,
  MessageSquare,
  BookOpen,
  Network,
  ShieldCheck,
  Activity,
}

export function ActionCard({ action }: { action: HomeAction }) {
  const Icon = IconMap[action.icon] || Search
  const isPrimary = action.variant === "primary"

  return (
    <Link 
      href={action.href}
      className={cn(
        "group relative flex flex-col p-6 rounded-2xl border transition-all duration-300",
        isPrimary 
          ? "bg-indigo-600/10 border-indigo-500/50 hover:bg-indigo-600/20 hover:border-indigo-400" 
          : "bg-zinc-900 border-zinc-800 hover:bg-zinc-800 hover:border-zinc-600"
      )}
    >
      <div className="flex items-center gap-4 mb-4">
        <div className={cn(
          "p-3 rounded-xl",
          isPrimary ? "bg-indigo-500/20 text-indigo-400" : "bg-zinc-800 text-zinc-400 group-hover:text-zinc-200"
        )}>
          <Icon className="w-6 h-6" />
        </div>
        <h3 className={cn(
          "text-lg font-semibold",
          isPrimary ? "text-indigo-300" : "text-zinc-100"
        )}>
          {action.title}
        </h3>
      </div>
      <p className="text-zinc-400 text-sm leading-relaxed mb-6 flex-grow">
        {action.description}
      </p>
      <div className="flex flex-wrap gap-2 mt-auto">
        {action.badges?.map(badge => (
          <span 
            key={badge} 
            className="px-2.5 py-1 text-xs font-mono rounded-md bg-zinc-950 border border-zinc-800 text-zinc-500"
          >
            {badge}
          </span>
        ))}
      </div>
    </Link>
  )
}
