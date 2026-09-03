import { create } from 'zustand'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

export interface DriveInfo {
  id: string
  path: string
  label: string | null
  drive_type: string
  has_disc: boolean
  disc_present: boolean
}

export interface Job {
  id: string
  name: string
  status: string
  progress: {
    job_id: string
    phase: string
    bytes_processed: number
    bytes_total: number
    files_processed: number
    files_total: number
    speed_bytes_per_sec: number
    eta_seconds: number | null
    started_at: string
    updated_at: string
  }
  created_at: string
}

export interface SystemInfo {
  num_cpus: number
  num_physical_cpus: number
  total_memory_gb: number
  available_memory_gb: number
  gpu_info: string[]
  os_name: string
  hostname: string
}

export interface AppSettings {
  default_output_dir: string
  read_speed: number | null
  read_retries: number
  buffer_size_mb: number
  verify_checksums: boolean
  eject_after_rip: boolean
  enable_audio_cd: boolean
  jitter_correction: boolean
  log_level: string
  auto_organize: boolean
  theme: 'dark' | 'light'
}

export const DEFAULT_SETTINGS: AppSettings = {
  default_output_dir: '',
  read_speed: null,
  read_retries: 3,
  buffer_size_mb: 8,
  verify_checksums: true,
  eject_after_rip: false,
  enable_audio_cd: true,
  jitter_correction: true,
  log_level: 'info',
  auto_organize: true,
  theme: 'dark',
}

export interface AppState {
  drives: DriveInfo[]
  jobs: Job[]
  systemInfo: SystemInfo | null
  settings: AppSettings
  loading: boolean
  error: string | null
  
  initialize: () => Promise<void>
  refreshDrives: () => Promise<void>
  refreshJobs: () => Promise<void>
  startRip: (driveId: string, outputPath: string) => Promise<string>
  startExtraction: (driveId: string, outputPath: string) => Promise<string>
  cancelJob: (jobId: string) => Promise<void>
  analyzeDrive: (driveId: string) => Promise<any>
  loadSettings: () => Promise<void>
  saveSettings: (settings: AppSettings) => Promise<void>
  resetSettings: () => Promise<void>
  setTheme: (theme: 'dark' | 'light') => void
  clearError: () => void
}

export const useAppStore = create<AppState>((set, get) => ({
  drives: [],
  jobs: [],
  systemInfo: null,
  settings: { ...DEFAULT_SETTINGS, theme: (localStorage.getItem('diskripper-theme') as 'dark' | 'light') || 'dark' },
  loading: false,
  error: null,

  initialize: async () => {
    const savedTheme = localStorage.getItem('diskripper-theme') as 'dark' | 'light' | null
    const theme = savedTheme || 'dark'
    document.documentElement.classList.toggle('light-theme', theme === 'light')
    
    await get().loadSettings()
    await get().refreshDrives()
    await get().refreshJobs()

    try {
      await listen('job:update', () => {
        get().refreshJobs()
      })
    } catch (e) {
      console.log('Event listening not available')
    }
  },

  refreshDrives: async () => {
    set({ loading: true, error: null })
    try {
      const drives = await invoke<DriveInfo[]>('list_drives')
      set({ drives, loading: false })
    } catch (e) {
      set({ error: String(e), loading: false })
    }
  },

  refreshJobs: async () => {
    try {
      const jobs = await invoke<Job[]>('list_jobs')
      set({ jobs })
    } catch (e) {
      console.log('Failed to refresh jobs:', e)
    }
  },

  startRip: async (driveId: string, outputPath: string) => {
    const jobId = await invoke<string>('start_image_rip', { driveId, outputPath })
    await get().refreshJobs()
    return jobId
  },

  startExtraction: async (driveId: string, outputPath: string) => {
    const jobId = await invoke<string>('start_extraction', { driveId, outputPath })
    await get().refreshJobs()
    return jobId
  },

  cancelJob: async (jobId: string) => {
    await invoke('cancel_job', { jobId })
    await get().refreshJobs()
  },

  analyzeDrive: async (driveId: string) => {
    return await invoke('analyze_drive', { driveId })
  },

  loadSettings: async () => {
    try {
      const saved = localStorage.getItem('diskripper-settings')
      if (saved) {
        const parsed = JSON.parse(saved)
        set({ settings: { ...DEFAULT_SETTINGS, ...parsed } })
      }
    } catch (e) {
      console.log('Failed to load settings:', e)
    }
  },

  saveSettings: async (settings: AppSettings) => {
    localStorage.setItem('diskripper-settings', JSON.stringify(settings))
    set({ settings })
  },

  resetSettings: async () => {
    localStorage.removeItem('diskripper-settings')
    set({ settings: DEFAULT_SETTINGS })
  },

  setTheme: (theme: 'dark' | 'light') => {
    localStorage.setItem('diskripper-theme', theme)
    document.documentElement.classList.toggle('light-theme', theme === 'light')
    set((state) => ({ settings: { ...state.settings, theme } }))
  },

  clearError: () => set({ error: null }),
}))
