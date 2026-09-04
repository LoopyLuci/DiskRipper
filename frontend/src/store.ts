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
  error?: string
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
  total_memory_bytes: number
  available_memory_bytes: number
  gpu_devices: GpuInfo[]
  os_name: string
  hostname: string
}

export interface GpuInfo {
  name: string
  platform: string
  memory_bytes: number
}

export interface TrackInfo {
  track_number: number
  start_lba: number
  length_lba: number
  channels: number
  duration_seconds: number
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

export interface Toast {
  id: string
  message: string
  type: 'info' | 'success' | 'warning' | 'error'
}

export interface AppState {
  drives: DriveInfo[]
  jobs: Job[]
  systemInfo: SystemInfo | null
  settings: AppSettings
  toasts: Toast[]
  loading: boolean
  error: string | null
  outputPath: string
  audioTracks: TrackInfo[]
  
  initialize: () => Promise<void>
  refreshDrives: () => Promise<void>
  refreshJobs: () => Promise<void>
  startRip: (driveId: string, outputPath: string) => Promise<string>
  startImageRip: (driveId: string, outputPath?: string) => Promise<string>
  startExtraction: (driveId: string, outputPath: string) => Promise<string>
  cancelJob: (jobId: string) => Promise<void>
  removeJob: (jobId: string) => Promise<void>
  analyzeDrive: (driveId: string) => Promise<any>
  loadSettings: () => Promise<void>
  saveSettings: (settings: AppSettings) => Promise<void>
  resetSettings: () => Promise<void>
  setTheme: (theme: 'dark' | 'light') => void
  clearError: () => void
  addToast: (message: string, type?: 'info' | 'success' | 'warning' | 'error') => void
  removeToast: (id: string) => void
  verifyImageRip: (driveId: string, imagePath: string) => Promise<any>
  selectedDrive: string | null
  setSelectedDrive: (driveId: string | null) => void
  selectDrive: (driveId: string) => void
  setOutputPath: (path: string) => void
  loadAudioTracks: (driveId: string) => Promise<void>
  extractAudioTrack: (driveId: string, trackNumber: number, outputPath: string) => Promise<string>
}

export const useAppStore = create<AppState>((set, get) => ({
  drives: [],
  jobs: [],
  systemInfo: null,
  settings: { ...DEFAULT_SETTINGS, theme: (localStorage.getItem('diskripper-theme') as 'dark' | 'light') || 'dark' },
  toasts: [],
  loading: false,
  error: null,
  selectedDrive: null,
  outputPath: '',
  audioTracks: [],

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

  startImageRip: async (driveId: string, outputPath?: string) => {
    const out = outputPath || get().outputPath || `C:\\DiskRipper\\${driveId.replace(':', '')}.iso`
    const jobId = await invoke<string>('start_image_rip', { driveId, outputPath: out })
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

  removeJob: async (jobId: string) => {
    await invoke('remove_job', { jobId })
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

  addToast: (message: string, type: 'info' | 'success' | 'warning' | 'error' = 'info') => {
    const toast: Toast = { id: Date.now().toString(), message, type }
    set((state) => ({ toasts: [...state.toasts, toast] }))
    setTimeout(() => { get().removeToast(toast.id) }, 5000)
  },

  removeToast: (id: string) => {
    set((state) => ({ toasts: state.toasts.filter((t) => t.id !== id) }))
  },

  verifyImageRip: async (driveId: string, imagePath: string) => {
    return await invoke('verify_image', { driveId, imagePath })
  },

  setSelectedDrive: (driveId: string | null) => {
    set({ selectedDrive: driveId })
  },

  selectDrive: (driveId: string) => {
    set({ selectedDrive: driveId })
  },

  setOutputPath: (path: string) => {
    set({ outputPath: path })
  },

  loadAudioTracks: async (driveId: string) => {
    try {
      const tracks = await invoke<TrackInfo[]>('list_audio_tracks', { driveId })
      set({ audioTracks: tracks })
    } catch (e) {
      console.log('Failed to load audio tracks:', e)
    }
  },

  extractAudioTrack: async (driveId: string, trackNumber: number, outputPath: string) => {
    return await invoke<string>('extract_audio_track', { driveId, trackNumber, outputPath })
  },
}))
