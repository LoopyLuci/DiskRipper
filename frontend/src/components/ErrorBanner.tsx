import { AlertCircle, X } from 'lucide-react'
import { useAppStore } from '../store'

export function ErrorBanner() {
  const { error, clearError } = useAppStore()

  if (!error) return null

  return (
    <div className="bg-red-500/20 border border-red-500 text-red-200 px-4 py-3 flex items-center justify-between">
      <div className="flex items-center gap-2">
        <AlertCircle size={18} />
        <span className="text-sm">{error}</span>
      </div>
      <button
        onClick={clearError}
        className="p-1 hover:bg-red-500/30 rounded"
      >
        <X size={16} />
      </button>
    </div>
  )
}
