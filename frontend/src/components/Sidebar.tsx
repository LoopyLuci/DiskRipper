import { HardDrive, Disc3, Settings, Activity, FileCheck, Music, Brain } from 'lucide-react'
import type { View } from '../App'

interface SidebarProps {
  currentView: View
  onViewChange: (view: View) => void
}

export function Sidebar({ currentView, onViewChange }: SidebarProps) {
  const items: { id: View; label: string; icon: React.ReactNode }[] = [
    { id: 'drives', label: 'Drives', icon: <HardDrive size={20} /> },
    { id: 'jobs', label: 'Jobs', icon: <Activity size={20} /> },
    { id: 'audio', label: 'Audio CD', icon: <Music size={20} /> },
    { id: 'verify', label: 'Verify', icon: <FileCheck size={20} /> },
    { id: 'ml', label: 'ML Identify', icon: <Brain size={20} /> },
    { id: 'settings', label: 'Settings', icon: <Settings size={20} /> },
  ]

  return (
    <aside className="w-56 bg-[#1e293b] border-r border-[#334155] flex flex-col">
      <div className="p-4 border-b border-[#334155]">
        <div className="flex items-center gap-2">
          <Disc3 size={28} className="text-blue-500" />
          <h1 className="text-lg font-bold text-white">DiskRipper</h1>
        </div>
        <p className="text-xs text-slate-400 mt-1">Media Backup Suite</p>
      </div>
      <nav className="flex-1 p-2">
        {items.map((item) => (
          <button
            key={item.id}
            onClick={() => onViewChange(item.id)}
            className={`w-full flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium transition-colors ${
              currentView === item.id
                ? 'bg-blue-600 text-white'
                : 'text-slate-300 hover:bg-[#334155] hover:text-white'
            }`}
          >
            {item.icon}
            {item.label}
          </button>
        ))}
      </nav>
      <div className="p-4 border-t border-[#334155]">
        <p className="text-xs text-slate-500">v0.1.0</p>
      </div>
    </aside>
  )
}
