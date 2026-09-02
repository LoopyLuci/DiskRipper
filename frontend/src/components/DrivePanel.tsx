import { useState } from 'react'
import { Disc3, HardDrive, Play, FolderOpen, RefreshCw } from 'lucide-react'
import { useAppStore } from '../store'
import { Spinner } from './Spinner'

export function DrivePanel() {
  const { drives, selectedDrive, selectDrive, startImageRip, startExtraction, outputPath, setOutputPath, loading, refreshDrives } = useAppStore()
  const [mode, setMode] = useState<'image' | 'extract'>('image')

  const handleRip = async () => {
    if (!selectedDrive || !outputPath) return
    if (mode === 'image') {
      await startImageRip(selectedDrive, outputPath)
    } else {
      await startExtraction(selectedDrive, outputPath)
    }
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-xl font-bold text-white">Optical Drives</h2>
        <button
          onClick={refreshDrives}
          disabled={loading}
          className="flex items-center gap-2 px-3 py-1.5 bg-[#334155] hover:bg-[#475569] disabled:bg-slate-600 rounded-lg text-sm text-slate-200 transition-colors"
        >
          {loading ? <Spinner size={14} /> : <RefreshCw size={14} />}
          Refresh
        </button>
      </div>

      {drives.length === 0 ? (
        <div className="bg-[#1e293b] rounded-xl p-8 text-center border border-[#334155]">
          <Disc3 size={48} className="mx-auto text-slate-500 mb-4" />
          <p className="text-slate-400">No optical drives detected</p>
          <p className="text-xs text-slate-500 mt-2">Insert a disc and click Refresh</p>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {drives.map((drive) => (
            <button
              key={drive.id}
              onClick={() => selectDrive(drive.id)}
              className={`p-4 rounded-xl border text-left transition-all ${
                selectedDrive === drive.id
                  ? 'bg-blue-600/20 border-blue-500 ring-1 ring-blue-500'
                  : 'bg-[#1e293b] border-[#334155] hover:border-[#475569]'
              }`}
            >
              <div className="flex items-center gap-3">
                <HardDrive size={24} className={drive.has_disc ? 'text-green-400' : 'text-slate-500'} />
                <div>
                  <p className="font-medium text-white">{drive.label || drive.id}</p>
                  <p className="text-xs text-slate-400">{drive.path} • {drive.drive_type}</p>
                </div>
              </div>
              <div className="mt-2 flex items-center gap-2">
                <span className={`text-xs px-2 py-0.5 rounded-full ${
                  drive.has_disc ? 'bg-green-500/20 text-green-400' : 'bg-slate-500/20 text-slate-400'
                }`}>
                  {drive.has_disc ? 'Disc Present' : 'Empty'}
                </span>
              </div>
            </button>
          ))}
        </div>
      )}

      {selectedDrive && (
        <div className="bg-[#1e293b] rounded-xl p-5 border border-[#334155] space-y-4">
          <h3 className="text-sm font-semibold text-white">Rip Configuration</h3>
          
          <div className="flex gap-2">
            <button
              onClick={() => setMode('image')}
              className={`px-4 py-2 rounded-lg text-sm font-medium transition-colors ${
                mode === 'image' ? 'bg-blue-600 text-white' : 'bg-[#334155] text-slate-300 hover:bg-[#475569]'
              }`}
            >
              Create Image
            </button>
            <button
              onClick={() => setMode('extract')}
              className={`px-4 py-2 rounded-lg text-sm font-medium transition-colors ${
                mode === 'extract' ? 'bg-blue-600 text-white' : 'bg-[#334155] text-slate-300 hover:bg-[#475569]'
              }`}
            >
              Extract Files
            </button>
          </div>

          <div>
            <label className="text-xs text-slate-400 block mb-1">Output Path</label>
            <div className="flex gap-2">
              <input
                type="text"
                value={outputPath}
                onChange={(e) => setOutputPath(e.target.value)}
                placeholder="Select output directory..."
                className="flex-1 bg-[#0f172a] border border-[#334155] rounded-lg px-3 py-2 text-sm text-white placeholder-slate-500 focus:outline-none focus:border-blue-500"
              />
              <button className="px-3 py-2 bg-[#334155] hover:bg-[#475569] rounded-lg text-slate-300">
                <FolderOpen size={16} />
              </button>
            </div>
            {!outputPath && (
              <p className="text-xs text-amber-400 mt-1">Output path is required</p>
            )}
          </div>

          <button
            onClick={handleRip}
            disabled={loading || !outputPath}
            className="flex items-center gap-2 px-5 py-2.5 bg-blue-600 hover:bg-blue-700 disabled:bg-slate-600 disabled:cursor-not-allowed rounded-lg text-sm font-medium text-white transition-colors"
          >
            {loading ? (
              <>
                <Spinner size={16} />
                Starting...
              </>
            ) : (
              <>
                <Play size={16} />
                {mode === 'image' ? 'Create Image' : 'Extract Files'}
              </>
            )}
          </button>
        </div>
      )}
    </div>
  )
}
