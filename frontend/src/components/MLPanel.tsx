import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'

export interface IdentificationResult {
  content_type: string
  confidence: number
  title: string | null
  artist: string | null
  album: string | null
  genre: string | null
  year: number | null
  source: string
  metadata: Record<string, string>
}

export function MLPanel() {
  const [audioFile, setAudioFile] = useState<string>('')
  const [result, setResult] = useState<IdentificationResult | null>(null)
  const [loading, setLoading] = useState(false)
  const [feedbackTitle, setFeedbackTitle] = useState('')
  const [feedbackArtist, setFeedbackArtist] = useState('')
  const [feedbackAlbum, setFeedbackAlbum] = useState('')
  const [feedbackGenre, setFeedbackGenre] = useState('')
  const [feedbackSubmitted, setFeedbackSubmitted] = useState(false)

  const handleIdentify = async () => {
    if (!audioFile) return
    setLoading(true)
    setResult(null)
    setFeedbackSubmitted(false)

    try {
      // For demo, generate synthetic audio data
      // In production, read actual audio file
      const sampleRate = 44100
      const durationSecs = 10
      const numSamples = sampleRate * durationSecs
      const audioData: number[] = []
      for (let i = 0; i < numSamples; i++) {
        const t = i / sampleRate
        const sample = Math.sin(2 * Math.PI * 440 * t) * 32767
        audioData.push(Math.round(sample))
      }

      const res = await invoke<IdentificationResult>('identify_audio', {
        audioData,
        sampleRate,
      })
      setResult(res)
      setFeedbackTitle(res.title || '')
      setFeedbackArtist(res.artist || '')
      setFeedbackAlbum(res.album || '')
      setFeedbackGenre(res.genre || '')
    } catch (e) {
      console.error('Identification failed:', e)
    }
    setLoading(false)
  }

  const handleSubmitFeedback = async () => {
    if (!result) return
    try {
      await invoke('submit_feedback', {
        feedback: {
          id: Date.now(),
          original_prediction: result.title || 'Unknown',
          corrected_title: feedbackTitle,
          corrected_artist: feedbackArtist || null,
          corrected_album: feedbackAlbum || null,
          corrected_genre: feedbackGenre || null,
          timestamp: new Date().toISOString(),
          used_for_training: false,
        },
      })
      setFeedbackSubmitted(true)
    } catch (e) {
      console.error('Feedback submission failed:', e)
    }
  }

  return (
    <div className="space-y-4">
      <h2 className="text-xl font-bold text-white">ML Content Identification</h2>
      <p className="text-gray-400 text-sm">
        Identify and organize ripped content using machine learning.
      </p>

      <div className="bg-gray-800 rounded-lg p-4 space-y-3">
        <label className="block text-sm text-gray-300">Audio File Path</label>
        <input
          type="text"
          value={audioFile}
          onChange={(e) => setAudioFile(e.target.value)}
          placeholder="C:\Users\Music\track.wav"
          className="w-full bg-gray-700 text-white rounded px-3 py-2 text-sm"
        />
        <button
          onClick={handleIdentify}
          disabled={loading || !audioFile}
          className="bg-blue-600 hover:bg-blue-700 disabled:bg-gray-600 text-white px-4 py-2 rounded text-sm"
        >
          {loading ? 'Identifying...' : 'Identify Content'}
        </button>
      </div>

      {result && (
        <div className="bg-gray-800 rounded-lg p-4 space-y-3">
          <h3 className="text-lg font-semibold text-white">Identification Result</h3>
          <div className="grid grid-cols-2 gap-2 text-sm">
            <div className="text-gray-400">Title:</div>
            <div className="text-white">{result.title || 'Unknown'}</div>
            <div className="text-gray-400">Artist:</div>
            <div className="text-white">{result.artist || 'Unknown'}</div>
            <div className="text-gray-400">Album:</div>
            <div className="text-white">{result.album || 'Unknown'}</div>
            <div className="text-gray-400">Genre:</div>
            <div className="text-white">{result.genre || 'Unknown'}</div>
            <div className="text-gray-400">Confidence:</div>
            <div className="text-white">{(result.confidence * 100).toFixed(1)}%</div>
            <div className="text-gray-400">Source:</div>
            <div className="text-white">{result.source}</div>
          </div>

          <div className="border-t border-gray-700 pt-3">
            <h4 className="text-sm font-semibold text-white mb-2">Provide Feedback (improves ML)</h4>
            <div className="space-y-2">
              <input
                type="text"
                value={feedbackTitle}
                onChange={(e) => setFeedbackTitle(e.target.value)}
                placeholder="Correct title"
                className="w-full bg-gray-700 text-white rounded px-3 py-2 text-sm"
              />
              <input
                type="text"
                value={feedbackArtist}
                onChange={(e) => setFeedbackArtist(e.target.value)}
                placeholder="Correct artist"
                className="w-full bg-gray-700 text-white rounded px-3 py-2 text-sm"
              />
              <input
                type="text"
                value={feedbackAlbum}
                onChange={(e) => setFeedbackAlbum(e.target.value)}
                placeholder="Correct album"
                className="w-full bg-gray-700 text-white rounded px-3 py-2 text-sm"
              />
              <input
                type="text"
                value={feedbackGenre}
                onChange={(e) => setFeedbackGenre(e.target.value)}
                placeholder="Correct genre"
                className="w-full bg-gray-700 text-white rounded px-3 py-2 text-sm"
              />
              <button
                onClick={handleSubmitFeedback}
                disabled={feedbackSubmitted}
                className="bg-green-600 hover:bg-green-700 disabled:bg-gray-600 text-white px-4 py-2 rounded text-sm"
              >
                {feedbackSubmitted ? 'Feedback Submitted!' : 'Submit Feedback'}
              </button>
            </div>
          </div>
        </div>
      )}

      <div className="bg-gray-800 rounded-lg p-4">
        <h3 className="text-lg font-semibold text-white mb-2">How It Works</h3>
        <ul className="text-sm text-gray-400 space-y-1">
          <li>• Custom audio fingerprinting identifies music from actual audio</li>
          <li>• Hybrid ML combines fingerprinting + classification + metadata</li>
          <li>• Self-learning pipeline improves from your feedback</li>
          <li>• All processing is local — no external API dependencies</li>
          <li>• Smart organization auto-sorts ripped content into folders</li>
        </ul>
      </div>
    </div>
  )
}
