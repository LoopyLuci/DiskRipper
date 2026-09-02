import { Disc3, Cpu, HardDrive } from 'lucide-react'

export function StatusBar() {
  return (
    <footer className="h-7 bg-[#1e293b] border-t border-[#334155] flex items-center px-4 text-xs text-slate-400">
      <div className="flex items-center gap-4">
        <span className="flex items-center gap-1">
          <Disc3 size={12} />
          Ready
        </span>
        <span className="flex items-center gap-1">
          <Cpu size={12} />
          Idle
        </span>
        <span className="flex items-center gap-1">
          <HardDrive size={12} />
          0 drives
        </span>
      </div>
      <div className="ml-auto">
        <span>DiskRipper v0.1.0</span>
      </div>
    </footer>
  )
}
