import { useState } from 'react'
import { Send, X, Bug, Lightbulb, MessageSquare, Camera } from 'lucide-react'

interface FeedbackDialogProps {
  isOpen: boolean
  onClose: () => void
}

type FeedbackType = 'bug' | 'feature' | 'general'

export function FeedbackDialog({ isOpen, onClose }: FeedbackDialogProps) {
  const [feedbackType, setFeedbackType] = useState<FeedbackType>('bug')
  const [subject, setSubject] = useState('')
  const [description, setDescription] = useState('')
  const [email, setEmail] = useState('')
  const [includeSystemInfo, setIncludeSystemInfo] = useState(true)
  const [includeLogs, setIncludeLogs] = useState(true)
  const [submitting, setSubmitting] = useState(false)
  const [submitted, setSubmitted] = useState(false)

  if (!isOpen) return null

  const handleSubmit = async () => {
    if (!subject.trim() || !description.trim()) return

    setSubmitting(true)
    
    // In a real implementation, this would send to a backend
    const feedback = {
      id: crypto.randomUUID(),
      type: feedbackType,
      subject,
      description,
      email: email || undefined,
      system_info: includeSystemInfo ? {
        app_version: '0.1.0',
        os: navigator.platform,
        user_agent: navigator.userAgent,
        timestamp: new Date().toISOString()
      } : undefined,
      include_logs: includeLogs,
      created_at: new Date().toISOString()
    }

    // Simulate API call
    await new Promise(resolve => setTimeout(resolve, 1000))
    console.log('Feedback submitted:', feedback)
    
    setSubmitting(false)
    setSubmitted(true)
    
    // Close after showing success
    setTimeout(() => {
      setSubmitted(false)
      onClose()
      // Reset form
      setSubject('')
      setDescription('')
      setEmail('')
      setFeedbackType('bug')
    }, 2000)
  }

  const typeOptions: { id: FeedbackType; label: string; icon: React.ReactNode }[] = [
    { id: 'bug', label: 'Bug Report', icon: <Bug size={16} /> },
    { id: 'feature', label: 'Feature Request', icon: <Lightbulb size={16} /> },
    { id: 'general', label: 'General Feedback', icon: <MessageSquare size={16} /> }
  ]

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-[#1e293b] rounded-xl border border-[#334155] w-full max-w-lg mx-4 shadow-xl">
        <div className="flex items-center justify-between p-4 border-b border-[#334155]">
          <h2 className="text-lg font-semibold text-white">Send Feedback</h2>
          <button
            onClick={onClose}
            className="p-1 hover:bg-[#334155] rounded text-slate-400 hover:text-white"
          >
            <X size={20} />
          </button>
        </div>

        {submitted ? (
          <div className="p-8 text-center">
            <div className="text-green-400 text-5xl mb-4">✓</div>
            <h3 className="text-xl font-semibold text-white mb-2">Thank You!</h3>
            <p className="text-slate-400">Your feedback has been submitted.</p>
          </div>
        ) : (
          <div className="p-4 space-y-4">
            {/* Feedback Type */}
            <div>
              <label className="text-sm text-slate-400 block mb-2">Type</label>
              <div className="flex gap-2">
                {typeOptions.map((option) => (
                  <button
                    key={option.id}
                    onClick={() => setFeedbackType(option.id)}
                    className={`flex items-center gap-2 px-3 py-2 rounded-lg text-sm font-medium transition-colors ${
                      feedbackType === option.id
                        ? 'bg-blue-600 text-white'
                        : 'bg-[#0f172a] text-slate-300 hover:bg-[#334155]'
                    }`}
                  >
                    {option.icon}
                    {option.label}
                  </button>
                ))}
              </div>
            </div>

            {/* Subject */}
            <div>
              <label className="text-sm text-slate-400 block mb-1">Subject</label>
              <input
                type="text"
                value={subject}
                onChange={(e) => setSubject(e.target.value)}
                placeholder="Brief summary of your feedback..."
                className="w-full bg-[#0f172a] border border-[#334155] rounded-lg px-3 py-2 text-sm text-white placeholder-slate-500 focus:outline-none focus:border-blue-500"
                maxLength={200}
              />
            </div>

            {/* Description */}
            <div>
              <label className="text-sm text-slate-400 block mb-1">Description</label>
              <textarea
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                placeholder="Please provide details..."
                rows={4}
                className="w-full bg-[#0f172a] border border-[#334155] rounded-lg px-3 py-2 text-sm text-white placeholder-slate-500 focus:outline-none focus:border-blue-500 resize-none"
                maxLength={5000}
              />
              <div className="text-xs text-slate-500 mt-1">
                {description.length}/5000 characters
              </div>
            </div>

            {/* Email (optional) */}
            <div>
              <label className="text-sm text-slate-400 block mb-1">Email (optional)</label>
              <input
                type="email"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                placeholder="For follow-up communication..."
                className="w-full bg-[#0f172a] border border-[#334155] rounded-lg px-3 py-2 text-sm text-white placeholder-slate-500 focus:outline-none focus:border-blue-500"
              />
            </div>

            {/* Options */}
            <div className="space-y-2">
              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="checkbox"
                  checked={includeSystemInfo}
                  onChange={(e) => setIncludeSystemInfo(e.target.checked)}
                  className="w-4 h-4 rounded border-[#334155] bg-[#0f172a] text-blue-500 focus:ring-blue-500"
                />
                <span className="text-sm text-slate-300">Include system information</span>
              </label>
              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="checkbox"
                  checked={includeLogs}
                  onChange={(e) => setIncludeLogs(e.target.checked)}
                  className="w-4 h-4 rounded border-[#334155] bg-[#0f172a] text-blue-500 focus:ring-blue-500"
                />
                <span className="text-sm text-slate-300">Include recent log entries</span>
              </label>
            </div>

            {/* Privacy Notice */}
            <p className="text-xs text-slate-500">
              Your feedback helps improve DiskRipper. No personal data is collected without your consent. 
              See our Privacy Policy for details.
            </p>

            {/* Actions */}
            <div className="flex justify-end gap-3 pt-2">
              <button
                onClick={onClose}
                className="px-4 py-2 bg-[#334155] hover:bg-[#475569] rounded-lg text-sm font-medium text-slate-300 transition-colors"
              >
                Cancel
              </button>
              <button
                onClick={handleSubmit}
                disabled={!subject.trim() || !description.trim() || submitting}
                className="flex items-center gap-2 px-4 py-2 bg-blue-600 hover:bg-blue-700 disabled:bg-slate-600 disabled:cursor-not-allowed rounded-lg text-sm font-medium text-white transition-colors"
              >
                {submitting ? (
                  <>
                    <div className="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                    Submitting...
                  </>
                ) : (
                  <>
                    <Send size={16} />
                    Submit Feedback
                  </>
                )}
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  )
}
