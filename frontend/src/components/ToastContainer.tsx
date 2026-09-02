import { AlertCircle, X, CheckCircle, Info } from 'lucide-react'
import { useAppStore } from '../store'

export function ToastContainer() {
  const { toasts, removeToast } = useAppStore()

  if (toasts.length === 0) return null

  return (
    <div className="fixed bottom-4 right-4 z-50 space-y-2">
      {toasts.map((toast) => (
        <div
          key={toast.id}
          className={`flex items-center gap-3 px-4 py-3 rounded-lg shadow-lg min-w-[300px] max-w-[400px] animate-slide-in ${
            toast.type === 'success'
              ? 'bg-green-500/20 border border-green-500 text-green-200'
              : toast.type === 'error'
              ? 'bg-red-500/20 border border-red-500 text-red-200'
              : 'bg-blue-500/20 border border-blue-500 text-blue-200'
          }`}
        >
          {toast.type === 'success' ? (
            <CheckCircle size={18} />
          ) : toast.type === 'error' ? (
            <AlertCircle size={18} />
          ) : (
            <Info size={18} />
          )}
          <span className="text-sm flex-1">{toast.message}</span>
          <button
            onClick={() => removeToast(toast.id)}
            className="p-1 hover:bg-white/10 rounded"
          >
            <X size={14} />
          </button>
        </div>
      ))}
    </div>
  )
}
