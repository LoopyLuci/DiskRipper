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
  updated_at: string
  error: string | null
}

export interface AudioTrack {
  track_number: number
  start_sector: number
  end_sector: number
  duration_seconds: number
  is_audio: boolean
}

export interface Settings {
  default_output_dir: string
  read_speed: number | null
  verify_checksums: boolean
  eject_after_rip: boolean
  read_retries: number
  buffer_size_mb: number
  log_level: string
  enable_audio_cd: boolean
  jitter_correction: boolean
  max_concurrent_jobs: number
  enable_parallel: boolean
  theme: string
  language: string
}

export interface SystemInfo {
  num_cpus: number
  num_physical_cpus: number
  total_memory_bytes: number
  available_memory_bytes: number
  gpu_devices: GpuDevice[]
}

export interface GpuDevice {
  name: string
  vendor: string
  memory_bytes: number
  platform: string
}

export interface Toast {
  id: string
  type: 'success' | 'error' | 'info'
  message: string
}

interface AppState {
  drives: DriveInfo[]
  jobs: Job[]
  selectedDrive: string | null
  outputPath: string
  loading: boolean
  error: string | null
  settings: Settings | null
  systemInfo: SystemInfo | null
  audioTracks: AudioTrack[]
  toasts: Toast[]
  
  refreshDrives: () => Promise<void>
  refreshJobs: () => Promise<void>
  selectDrive: (id: string | null) => void
  setOutputPath: (path: string) => void
  setError: (error: string | null) => void
  clearError: () => void
  
  startImageRip: (driveId: string, outputPath: string) => Promise<string | null>
  startExtraction: (driveId: string, outputPath: string) => Promise<string | null>
  cancelJob: (jobId: string) => Promise<void>
  removeJob: (jobId: string) => Promise<void>
  
  loadSettings: () => Promise<void>
  saveSettings: (settings: Settings) => Promise<void>
  resetSettings: () => Promise<void>
  loadSystemInfo: () => Promise<void>
  
  loadAudioTracks: (driveId: string) => Promise<void>
  extractAudioTrack: (driveId: string, trackNumber: number, outputPath: string) => Promise<void>
  
  verifyImageRip: (driveId: string, imagePath: string) => Promise<any>
  
  addToast: (type: Toast['type'], message: string) => void
  removeToast: (id: string) => void
  
  initialize: () => Promise<void>
}

export const useAppStore = create<AppState>((set, get) => ({
  drives: [],
  jobs: [],
  selectedDrive: null,
  outputPath: '',
  loading: false,
  error: null,
  settings: null,
  systemInfo: null,
  audioTracks: [],
  toasts: [],

  refreshDrives: async () => {
    try {
      const drives = await invoke<DriveInfo[]>('list_drives')
      set({ drives, error: null })
    } catch (e) {
      set({ error: String(e) })
    }
  },

  refreshJobs: async () => {
    try {
      const jobs = await invoke<Job[]>('list_jobs')
      set({ jobs, error: null })
    } catch (e) {
      set({ error: String(e) })
    }
  },

  selectDrive: (id) => set({ selectedDrive: id }),
  
  setOutputPath: (path) => set({ outputPath: path }),
  
  setError: (error) => set({ error }),
  
  clearError: () => set({ error: null }),

  startImageRip: async (driveId, outputPath) => {
    set({ loading: true, error: null })
    try {
      if (!outputPath.trim()) {
        throw new Error('Output path is required')
      }
      if (!driveId) {
        throw new Error('No drive selected')
      }
      
      const jobId = await invoke<string>('start_image_rip', { driveId, outputPath })
      await get().refreshJobs()
      set({ loading: false })
      get().addToast('info', 'Image rip started')
      return jobId
    } catch (e) {
      set({ loading: false, error: String(e) })
      get().addToast('error', `Failed to start rip: ${e}`)
      return null
    }
  },

  startExtraction: async (driveId, outputPath) => {
    set({ loading: true, error: null })
    try {
      if (!outputPath.trim()) {
        throw new Error('Output path is required')
      }
      if (!driveId) {
        throw new Error('No drive selected')
      }
      
      const jobId = await invoke<string>('start_extraction', { driveId, outputPath })
      await get().refreshJobs()
      set({ loading: false })
      get().addToast('info', 'Extraction started')
      return jobId
    } catch (e) {
      set({ loading: false, error: String(e) })
      get().addToast('error', `Failed to start extraction: ${e}`)
      return null
    }
  },

  cancelJob: async (jobId) => {
    try {
      await invoke('cancel_job', { jobId })
      await get().refreshJobs()
      get().addToast('info', 'Job cancelled')
    } catch (e) {
      set({ error: String(e) })
      get().addToast('error', `Failed to cancel job: ${e}`)
    }
  },

  removeJob: async (jobId) => {
    try {
      await invoke('remove_job', { jobId })
      await get().refreshJobs()
    } catch (e) {
      set({ error: String(e) })
    }
  },

  loadSettings: async () => {
    try {
      const settings = await invoke<Settings>('load_settings')
      set({ settings })
    } catch (e) {
      set({ error: String(e) })
    }
  },

  saveSettings: async (settings) => {
    try {
      await invoke('save_settings', { settings })
      set({ settings })
      get().addToast('success', 'Settings saved')
    } catch (e) {
      set({ error: String(e) })
      get().addToast('error', `Failed to save settings: ${e}`)
    }
  },

  resetSettings: async () => {
    try {
      const settings = await invoke<Settings>('reset_settings')
      set({ settings })
      get().addToast('success', 'Settings reset to defaults')
    } catch (e) {
      set({ error: String(e) })
    }
  },

  loadSystemInfo: async () => {
    try {
      const info = await invoke<SystemInfo>('get_system_info')
      set({ systemInfo: info })
    } catch (e) {
      // System info is optional, don't show error
      console.debug('System info not available:', e)
    }
  },

  loadAudioTracks: async (driveId) => {
    try {
      const tracks = await invoke<AudioTrack[]>('get_audio_tracks', { driveId })
      set({ audioTracks: tracks })
    } catch (e) {
      set({ error: String(e) })
    }
  },

  extractAudioTrack: async (driveId, trackNumber, outputPath) => {
    set({ loading: true, error: null })
    try {
      if (!outputPath.trim()) {
        throw new Error('Output path is required')
      }
      await invoke('extract_audio_track_to_wav', { driveId, trackNumber, outputPath })
      set({ loading: false })
      get().addToast('success', `Track ${trackNumber} extracted`)
    } catch (e) {
      set({ loading: false, error: String(e) })
      get().addToast('error', `Failed to extract track: ${e}`)
    }
  },

  verifyImageRip: async (driveId, imagePath) => {
    set({ loading: true, error: null })
    try {
      const result = await invoke('verify_image_rip', { driveId, imagePath })
      set({ loading: false })
      get().addToast('success', 'Verification complete')
      return result
    } catch (e) {
      set({ loading: false, error: String(e) })
      get().addToast('error', `Verification failed: ${e}`)
      return null
    }
  },

  addToast: (type, message) => {
    const id = crypto.randomUUID()
    set((state) => ({ toasts: [...state.toasts, { id, type, message }] }))
    setTimeout(() => {
      get().removeToast(id)
    }, 5000)
  },

  removeToast: (id) => {
    set((state) => ({ toasts: state.toasts.filter((t) => t.id !== id) }))
  },

  initialize: async () => {
    await get().refreshDrives()
    await get().refreshJobs()
    await get().loadSettings()
    await get().loadSystemInfo()
    
    const settings = get().settings
    if (settings) {
      set({ outputPath: settings.default_output_dir })
    } else {
      const defaultPath = await invoke<string>('get_default_output_path')
      set({ outputPath: defaultPath })
    }
    
    // Listen for job events from backend
    await listen('job:update', () => {
      get().refreshJobs()
    })
    
    await listen('job:completed', (event) => {
      get().refreshJobs()
      get().addToast('success', 'Job completed')
    })
    
    await listen('job:failed', (event) => {
      get().refreshJobs()
      get().addToast('error', `Job failed: ${event.payload}`)
    })
  },
}))
