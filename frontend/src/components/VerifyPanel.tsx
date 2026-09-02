import { useState } from 'react'
import { FileCheck, CheckCircle, XCircle, RefreshCw } from 'lucide-react'
import { useAppStore } from '../store'

export function VerifyPanel() {
  const { drives, selectedDrive, loading, verifyImageRip } = useAppStore()
  const [imagePath, setImagePath] = useState('')
  const [results, setResults] = useState<any>(null)
  const [verifying, setVerifying] = useState(false)

  const handleVerify = async () => {
    if (selectedDrive && imagePath) {
      setVerifying(true)
      try {
        const result = await verifyImageRip(selectedDrive, imagePath)
        setResults(result)
      } catch (e) {
        console.error(e)
      }
      setVerifying(false)
    }
  }

  return (
    <div className="space-y-6">
      <h2 className="text-xl font-bold text-white">Verify Rip</h2>
      <p className="text-sm text-slate-400">
        Verify an image file matches the source disc by comparing SHA-256 checksums sector by sector.
      </p>

      {!selectedDrive ? (
        <div className="bg-[#1e293b] rounded-xl p-8 text-center border border-[#334155]">
          <FileCheck size={48} className="mx-auto text-slate-500 mb-4" />
          <p className="text-slate-400">Select a drive to verify against</p>
        </div>
      ) : (
        <div className="space-y-4">
          <div className="bg-[#1e293b] rounded-xl p-4 border border-[#334155] space-y-4">
            <div>
              <label className="text-xs text-slate-400 block mb-1">Image File Path</label>
              <input
                type="text"
                value={imagePath}
                onChange={(e) => setImagePath(e.target.value)}
                placeholder="Path to ISO/image file..."
                className="w-full bg-[#0f172a] border border-[#334155] rounded-lg px-3 py-2 text-sm text-white placeholder-slate-500 focus:outline-none focus:border-blue-500"
              />
            </div>
            <button
              onClick={handleVerify}
              disabled={!imagePath || verifying}
              className="flex items-center gap-2 px-5 py-2.5 bg-blue-600 hover:bg-blue-700 disabled:bg-slate-600 disabled:cursor-not-allowed rounded-lg text-sm font-medium text-white transition-colors"
            >
              {verifying ? (
                <>
                  <RefreshCw size={16} className="animate-spin" />
                  Verifying...
                </>
              ) : (
                <>
                  <FileCheck size={16} />
                  Verify Image
                </>
              )}
            </button>
          </div>

          {results && (
            <div className="bg-[#1e293b] rounded-xl p-4 border border-[#334155]">
              <h3 className="text-sm font-semibold text-white mb-3">Verification Results</h3>
              <div className="space-y-2 max-h-96 overflow-auto">
                {results.map((result: any, index: number) => (
                  <div
                    key={index}
                    className={`flex items-center justify-between p-3 rounded-lg ${
                      result.valid ? 'bg-green-500/10' : 'bg-red-500/10'
                    }`}
                  >
                    <div className="flex items-center gap-2">
                      {result.valid ? (
                        <CheckCircle size={16} className="text-green-400" />
                      ) : (
                        <XCircle size={16} className="text-red-400" />
                      )}
                      <span className="text-sm text-white">{result.file_path}</span>
                    </div>
                    <span className="text-xs text-slate-400">
                      {(result.bytes_verified / 1024).toFixed(0)} KB
                    </span>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  )
}
