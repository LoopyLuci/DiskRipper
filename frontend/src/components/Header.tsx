import { Disc3 } from 'lucide-react'

export function Header() {
  return (
    <header className="h-12 bg-[#1e293b] border-b border-[#334155] flex items-center px-4">
      <div className="flex items-center gap-2">
        <Disc3 size={18} className="text-blue-500" />
        <span className="text-sm font-semibold text-white">DiskRipper</span>
        <span className="text-xs text-slate-400 ml-2">Next-Gen Media Backup</span>
      </div>
    </header>
  )
}
