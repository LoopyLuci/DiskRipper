import { useState, useEffect } from 'react'
import { Save, FolderOpen, RotateCcw } from 'lucide-react'
import { useAppStore } from '../store'

export function SettingsPanel() {
  const { settings, loadSettings, saveSettings, resetSettings } = useAppStore()
  const [localSettings, setLocalSettings] = useState(settings)
  const [saving, setSaving] = useState(false)

  useEffect(() => {
    if (!settings) {
      loadSettings()
    } else {
      setLocalSettings(settings)
    }
  }, [settings, loadSettings])

  const handleSave = async () => {
    if (localSettings) {
      setSaving(true)
      await saveSettings(localSettings)
      setSaving(false)
    }
  }

  const handleReset = async () => {
    await resetSettings()
  }

  if (!localSettings) {
    return (
      <div className="bg-[#1e293b] rounded-xl p-8 text-center border border-[#334155]">
        <p className="text-slate-400">Loading settings...</p>
      </div>
    )
  }

  return (
    <div className="space-y-6 max-w-2xl">
      <h2 className="text-xl font-bold text-white">Settings</h2>

      <div className="bg-[#1e293b] rounded-xl p-5 border border-[#334155] space-y-4">
        <h3 className="text-sm font-semibold text-white">Output</h3>
        <div>
          <label className="text-xs text-slate-400 block mb-1">Default Output Directory</label>
          <div className="flex gap-2">
            <input
              type="text"
              value={localSettings.default_output_dir}
              onChange={(e) => setLocalSettings({ ...localSettings, default_output_dir: e.target.value })}
              className="flex-1 bg-[#0f172a] border border-[#334155] rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-blue-500"
            />
          </div>
        </div>
      </div>

      <div className="bg-[#1e293b] rounded-xl p-5 border border-[#334155] space-y-4">
        <h3 className="text-sm font-semibold text-white">Reading</h3>
        <div>
          <label className="text-xs text-slate-400 block mb-1">Read Speed</label>
          <select
            value={localSettings.read_speed ?? 'max'}
            onChange={(e) => {
              const val = e.target.value
              setLocalSettings({ 
                ...localSettings, 
                read_speed: val === 'max' ? null : parseInt(val) 
              })
            }}
            className="w-full bg-[#0f172a] border border-[#334155] rounded-lg px-3 py-2 text-sm text-white"
          >
            <option value="max">Maximum</option>
            <option value="16">16x</option>
            <option value="8">8x</option>
            <option value="4">4x</option>
            <option value="1">1x</option>
          </select>
        </div>
        <div>
          <label className="text-xs text-slate-400 block mb-1">Retries on Error</label>
          <input
            type="number"
            value={localSettings.read_retries}
            onChange={(e) => setLocalSettings({ ...localSettings, read_retries: Number(e.target.value) })}
            min={0}
            max={10}
            className="w-full bg-[#0f172a] border border-[#334155] rounded-lg px-3 py-2 text-sm text-white"
          />
        </div>
        <div>
          <label className="text-xs text-slate-400 block mb-1">Buffer Size (MB)</label>
          <input
            type="number"
            value={localSettings.buffer_size_mb}
            onChange={(e) => setLocalSettings({ ...localSettings, buffer_size_mb: Number(e.target.value) })}
            min={1}
            max={64}
            className="w-full bg-[#0f172a] border border-[#334155] rounded-lg px-3 py-2 text-sm text-white"
          />
        </div>
      </div>

      <div className="bg-[#1e293b] rounded-xl p-5 border border-[#334155] space-y-4">
        <h3 className="text-sm font-semibold text-white">Options</h3>
        <label className="flex items-center gap-3 cursor-pointer">
          <input
            type="checkbox"
            checked={localSettings.verify_checksums}
            onChange={(e) => setLocalSettings({ ...localSettings, verify_checksums: e.target.checked })}
            className="w-4 h-4 rounded border-[#334155] bg-[#0f172a] text-blue-500 focus:ring-blue-500"
          />
          <span className="text-sm text-slate-300">Verify checksums after rip</span>
        </label>
        <label className="flex items-center gap-3 cursor-pointer">
          <input
            type="checkbox"
            checked={localSettings.eject_after_rip}
            onChange={(e) => setLocalSettings({ ...localSettings, eject_after_rip: e.target.checked })}
            className="w-4 h-4 rounded border-[#334155] bg-[#0f172a] text-blue-500 focus:ring-blue-500"
          />
          <span className="text-sm text-slate-300">Eject disc after completion</span>
        </label>
        <label className="flex items-center gap-3 cursor-pointer">
          <input
            type="checkbox"
            checked={localSettings.enable_audio_cd}
            onChange={(e) => setLocalSettings({ ...localSettings, enable_audio_cd: e.target.checked })}
            className="w-4 h-4 rounded border-[#334155] bg-[#0f172a] text-blue-500 focus:ring-blue-500"
          />
          <span className="text-sm text-slate-300">Enable audio CD extraction</span>
        </label>
        <label className="flex items-center gap-3 cursor-pointer">
          <input
            type="checkbox"
            checked={localSettings.jitter_correction}
            onChange={(e) => setLocalSettings({ ...localSettings, jitter_correction: e.target.checked })}
            className="w-4 h-4 rounded border-[#334155] bg-[#0f172a] text-blue-500 focus:ring-blue-500"
          />
          <span className="text-sm text-slate-300">Jitter correction (audio CDs)</span>
        </label>
      </div>

      <div className="bg-[#1e293b] rounded-xl p-5 border border-[#334155] space-y-4">
        <h3 className="text-sm font-semibold text-white">Logging</h3>
        <div>
          <label className="text-xs text-slate-400 block mb-1">Log Level</label>
          <select
            value={localSettings.log_level}
            onChange={(e) => setLocalSettings({ ...localSettings, log_level: e.target.value })}
            className="w-full bg-[#0f172a] border border-[#334155] rounded-lg px-3 py-2 text-sm text-white"
          >
            <option value="trace">Trace</option>
            <option value="debug">Debug</option>
            <option value="info">Info</option>
            <option value="warn">Warn</option>
            <option value="error">Error</option>
          </select>
        </div>
      </div>

      <div className="flex gap-3">
        <button
          onClick={handleSave}
          disabled={saving}
          className="flex items-center gap-2 px-5 py-2.5 bg-blue-600 hover:bg-blue-700 disabled:bg-slate-600 rounded-lg text-sm font-medium text-white transition-colors"
        >
          <Save size={16} />
          {saving ? 'Saving...' : 'Save Settings'}
        </button>
        <button
          onClick={handleReset}
          className="flex items-center gap-2 px-5 py-2.5 bg-[#334155] hover:bg-[#475569] rounded-lg text-sm font-medium text-slate-300 transition-colors"
        >
          <RotateCcw size={16} />
          Reset to Defaults
        </button>
      </div>
    </div>
  )
}
