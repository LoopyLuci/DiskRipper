import { useState } from 'react'
import { Music, Download, RefreshCw } from 'lucide-react'
import { useAppStore } from '../store'

export function AudioCdPanel() {
  const { drives, selectedDrive, audioTracks, loading, loadAudioTracks, extractAudioTrack, outputPath, setOutputPath } = useAppStore()
  const [selectedTrack, setSelectedTrack] = useState<number | null>(null)

  const handleLoadTracks = async () => {
    if (selectedDrive) {
      await loadAudioTracks(selectedDrive)
    }
  }

  const handleExtract = async () => {
    if (selectedDrive && selectedTrack !== null) {
      const track = audioTracks.find(t => t.track_number === selectedTrack)
      if (track) {
        const fileName = `track_${selectedTrack.toString().padStart(2, '0')}.wav`
        const fullPath = outputPath ? `${outputPath}/${fileName}` : fileName
        await extractAudioTrack(selectedDrive, selectedTrack, fullPath)
      }
    }
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-xl font-bold text-white">Audio CD Extraction</h2>
        <button
          onClick={handleLoadTracks}
          disabled={!selectedDrive || loading}
          className="flex items-center gap-2 px-3 py-1.5 bg-[#334155] hover:bg-[#475569] disabled:bg-slate-600 disabled:cursor-not-allowed rounded-lg text-sm text-slate-200 transition-colors"
        >
          <RefreshCw size={14} />
          Load Tracks
        </button>
      </div>

      {!selectedDrive ? (
        <div className="bg-[#1e293b] rounded-xl p-8 text-center border border-[#334155]">
          <Music size={48} className="mx-auto text-slate-500 mb-4" />
          <p className="text-slate-400">Select a drive with an audio CD to extract tracks</p>
        </div>
      ) : audioTracks.length === 0 ? (
        <div className="bg-[#1e293b] rounded-xl p-8 text-center border border-[#334155]">
          <Music size={48} className="mx-auto text-slate-500 mb-4" />
          <p className="text-slate-400">Click "Load Tracks" to read the CD Table of Contents</p>
        </div>
      ) : (
        <div className="space-y-4">
          <div className="bg-[#1e293b] rounded-xl p-4 border border-[#334155]">
            <h3 className="text-sm font-semibold text-white mb-3">Tracks</h3>
            <div className="space-y-2">
              {audioTracks.map((track) => (
                <button
                  key={track.track_number}
                  onClick={() => setSelectedTrack(track.track_number)}
                  className={`w-full flex items-center justify-between p-3 rounded-lg transition-colors ${
                    selectedTrack === track.track_number
                      ? 'bg-blue-600/20 border border-blue-500'
                      : 'bg-[#0f172a] hover:bg-[#334155]'
                  }`}
                >
                  <div className="flex items-center gap-3">
                    <Music size={16} className="text-slate-400" />
                    <span className="text-sm text-white">Track {track.track_number}</span>
                  </div>
                  <span className="text-xs text-slate-400">
                    {Math.floor(track.duration_seconds / 60)}:{Math.floor(track.duration_seconds % 60).toString().padStart(2, '0')}
                  </span>
                </button>
              ))}
            </div>
          </div>

          <div className="bg-[#1e293b] rounded-xl p-4 border border-[#334155] space-y-4">
            <h3 className="text-sm font-semibold text-white">Output</h3>
            <div>
              <label className="text-xs text-slate-400 block mb-1">Output Path</label>
              <input
                type="text"
                value={outputPath}
                onChange={(e) => setOutputPath(e.target.value)}
                placeholder="Select output directory..."
                className="w-full bg-[#0f172a] border border-[#334155] rounded-lg px-3 py-2 text-sm text-white placeholder-slate-500 focus:outline-none focus:border-blue-500"
              />
            </div>
            <button
              onClick={handleExtract}
              disabled={selectedTrack === null || loading || !outputPath}
              className="flex items-center gap-2 px-5 py-2.5 bg-blue-600 hover:bg-blue-700 disabled:bg-slate-600 disabled:cursor-not-allowed rounded-lg text-sm font-medium text-white transition-colors"
            >
              <Download size={16} />
              {loading ? 'Extracting...' : 'Extract Track to WAV'}
            </button>
          </div>
        </div>
      )}
    </div>
  )
}
