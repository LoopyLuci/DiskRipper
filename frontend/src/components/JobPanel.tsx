import { useState, useEffect } from 'react'
import { Play, Trash2, X, CheckCircle, AlertCircle, Clock, Loader, Zap, HardDrive } from 'lucide-react'
import { useAppStore, type Job } from '../store'

function StatusIcon({ status }: { status: string }) {
  switch (status) {
    case 'Completed':
      return <CheckCircle size={16} className="text-green-400" />
    case 'Failed':
    case 'Error':
      return <AlertCircle size={16} className="text-red-400" />
    case 'Running':
      return <Loader size={16} className="text-blue-400 animate-spin" />
    case 'Cancelled':
      return <X size={16} className="text-slate-400" />
    default:
      return <Clock size={16} className="text-slate-400" />
  }
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`
}

function formatDuration(seconds: number): string {
  if (seconds < 60) return `${Math.floor(seconds)}s`
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ${Math.floor(seconds % 60)}s`
  return `${Math.floor(seconds / 3600)}h ${Math.floor((seconds % 3600) / 60)}m`
}

function ProgressBar({ progress }: { progress: Job['progress'] }) {
  const percent = progress.bytes_total > 0
    ? Math.min(100, (progress.bytes_processed / progress.bytes_total) * 100)
    : 0

  const speed = progress.speed_bytes_per_sec
  const eta = progress.eta_seconds

  return (
    <div className="mt-3">
      <div className="flex justify-between text-xs text-slate-400 mb-1">
        <span className="flex items-center gap-1">
          <Zap size={10} className="text-yellow-400" />
          {progress.phase}
        </span>
        <span className="font-mono">{percent.toFixed(1)}%</span>
      </div>
      <div className="h-2 bg-[#334155] rounded-full overflow-hidden">
        <div
          className="h-full bg-gradient-to-r from-blue-500 to-cyan-400 rounded-full transition-all duration-300"
          style={{ width: `${percent}%` }}
        />
      </div>
      <div className="grid grid-cols-3 gap-2 mt-2 text-xs">
        <div className="bg-[#0f172a] rounded px-2 py-1">
          <div className="text-slate-500">Processed</div>
          <div className="text-slate-200 font-mono">{formatBytes(progress.bytes_processed)}</div>
        </div>
        <div className="bg-[#0f172a] rounded px-2 py-1">
          <div className="text-slate-500">Speed</div>
          <div className="text-green-400 font-mono">
            {speed > 0 ? `${formatBytes(speed)}/s` : '-'}
          </div>
        </div>
        <div className="bg-[#0f172a] rounded px-2 py-1">
          <div className="text-slate-500">ETA</div>
          <div className="text-cyan-400 font-mono">
            {eta && eta > 0 ? formatDuration(eta) : '-'}
          </div>
        </div>
      </div>
    </div>
  )
}

function SystemInfo() {
  const { systemInfo } = useAppStore()
  
  if (!systemInfo) return null
  
  return (
    <div className="bg-[#1e293b] rounded-xl p-4 border border-[#334155]">
      <h3 className="text-sm font-semibold text-white mb-3 flex items-center gap-2">
        <HardDrive size={14} />
        System
      </h3>
      <div className="grid grid-cols-2 gap-2 text-xs">
        <div className="bg-[#0f172a] rounded px-2 py-1.5">
          <div className="text-slate-500">CPU Cores</div>
          <div className="text-slate-200">{systemInfo.num_cpus} logical / {systemInfo.num_physical_cpus} physical</div>
        </div>
        <div className="bg-[#0f172a] rounded px-2 py-1.5">
          <div className="text-slate-500">Memory</div>
          <div className="text-slate-200">{formatBytes(systemInfo.total_memory_bytes)} total</div>
        </div>
        {systemInfo.gpu_devices.length > 0 && (
          <div className="col-span-2 bg-[#0f172a] rounded px-2 py-1.5">
            <div className="text-slate-500">GPU</div>
            <div className="text-slate-200">
              {systemInfo.gpu_devices.map(g => 
                `${g.name} (${g.platform}, ${formatBytes(g.memory_bytes)})`
              ).join(', ')}
            </div>
          </div>
        )}
      </div>
    </div>
  )
}

export function JobPanel() {
  const { jobs, cancelJob, removeJob, refreshJobs } = useAppStore()
  const [filter, setFilter] = useState<string>('all')

  const filteredJobs = jobs.filter((job) => {
    if (filter === 'all') return true
    return job.status.toLowerCase() === filter
  })

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-xl font-bold text-white">Jobs</h2>
        <div className="flex gap-2">
          <select
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            className="bg-[#334155] border border-[#475569] rounded-lg px-3 py-1.5 text-sm text-slate-200"
          >
            <option value="all">All</option>
            <option value="running">Running</option>
            <option value="completed">Completed</option>
            <option value="failed">Failed</option>
            <option value="queued">Queued</option>
          </select>
          <button
            onClick={refreshJobs}
            className="px-3 py-1.5 bg-[#334155] hover:bg-[#475569] rounded-lg text-sm text-slate-200"
          >
            Refresh
          </button>
        </div>
      </div>

      <SystemInfo />

      {filteredJobs.length === 0 ? (
        <div className="bg-[#1e293b] rounded-xl p-8 text-center border border-[#334155]">
          <Clock size={48} className="mx-auto text-slate-500 mb-4" />
          <p className="text-slate-400">No jobs found</p>
          <p className="text-xs text-slate-500 mt-2">Start a rip from the Drives panel</p>
        </div>
      ) : (
        <div className="space-y-3">
          {filteredJobs.map((job) => (
            <div
              key={job.id}
              className="bg-[#1e293b] rounded-xl p-4 border border-[#334155]"
            >
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-3">
                  <StatusIcon status={job.status} />
                  <div>
                    <p className="text-sm font-medium text-white">{job.name}</p>
                    <p className="text-xs text-slate-400">
                      {new Date(job.created_at).toLocaleString()}
                    </p>
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  <span className={`text-xs px-2 py-0.5 rounded-full ${
                    job.status === 'Completed' ? 'bg-green-500/20 text-green-400' :
                    job.status === 'Failed' ? 'bg-red-500/20 text-red-400' :
                    job.status === 'Running' ? 'bg-blue-500/20 text-blue-400' :
                    'bg-slate-500/20 text-slate-400'
                  }`}>
                    {job.status}
                  </span>
                  {job.status === 'Running' && (
                    <button
                      onClick={() => cancelJob(job.id)}
                      className="p-1.5 bg-red-500/20 hover:bg-red-500/30 rounded-lg text-red-400"
                    >
                      <X size={14} />
                    </button>
                  )}
                  {(job.status === 'Completed' || job.status === 'Failed' || job.status === 'Cancelled') && (
                    <button
                      onClick={() => removeJob(job.id)}
                      className="p-1.5 bg-[#334155] hover:bg-[#475569] rounded-lg text-slate-400"
                    >
                      <Trash2 size={14} />
                    </button>
                  )}
                </div>
              </div>
              {job.status === 'Running' && <ProgressBar progress={job.progress} />}
              {job.error && (
                <p className="mt-2 text-xs text-red-400 bg-red-500/10 rounded-lg px-3 py-2">
                  {job.error}
                </p>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
