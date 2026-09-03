import { MLPanel } from './components/MLPanel'
import { useState, useEffect } from 'react'
import { Header } from './components/Header'
import { Sidebar } from './components/Sidebar'
import { DrivePanel } from './components/DrivePanel'
import { JobPanel } from './components/JobPanel'
import { SettingsPanel } from './components/SettingsPanel'
import { StatusBar } from './components/StatusBar'
import { ErrorBanner } from './components/ErrorBanner'
import { ToastContainer } from './components/ToastContainer'
import { AudioCdPanel } from './components/AudioCdPanel'
import { VerifyPanel } from './components/VerifyPanel'
import { FeedbackDialog } from './components/FeedbackDialog'
import { useAppStore } from './store'

export type View = 'drives' | 'jobs' | 'audio' | 'verify' | 'ml' | 'settings'

function App() {
  const [currentView, setCurrentView] = useState<View>('drives')
  const [feedbackOpen, setFeedbackOpen] = useState(false)
  const { initialize } = useAppStore()

  useEffect(() => {
    initialize()
  }, [initialize])

  // Keyboard shortcuts
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.key === 'r') {
        e.preventDefault()
        useAppStore.getState().refreshDrives()
      }
      if (e.ctrlKey && e.key === 'j') {
        e.preventDefault()
        setCurrentView('jobs')
      }
      if (e.ctrlKey && e.key === ',') {
        e.preventDefault()
        setCurrentView('settings')
      }
      if (e.ctrlKey && e.shiftKey && e.key === 'F') {
        e.preventDefault()
        setFeedbackOpen(true)
      }
      if (e.key === 'Escape') {
        setFeedbackOpen(false)
      }
    }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [])

  return (
    <div className="flex flex-col h-screen bg-[#0f172a]">
      <Header />
      <ErrorBanner />
      <div className="flex flex-1 overflow-hidden">
        <Sidebar currentView={currentView} onViewChange={setCurrentView} />
        <main className="flex-1 overflow-auto p-6">
          {currentView === 'drives' && <DrivePanel />}
          {currentView === 'jobs' && <JobPanel />}
          {currentView === 'audio' && <AudioCdPanel />}
          {currentView === 'verify' && <VerifyPanel />}
          {currentView === 'ml' && <MLPanel />}
          {currentView === 'settings' && <SettingsPanel />}
        </main>
      </div>
      <StatusBar />
      <ToastContainer />
      <FeedbackDialog isOpen={feedbackOpen} onClose={() => setFeedbackOpen(false)} />
    </div>
  )
}

export default App
