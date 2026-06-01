// API key setup onboarding screen
// Part of PR5b (AI UI)

import { useState } from 'react'
import { configureAI } from '../../lib/tauri-api'

interface ApiKeySetupProps {
  onConfigured?: () => void
  onSkip?: () => void
}

const PROVIDER_OPTIONS = [
  { value: 'anthropic', label: 'Anthropic (Claude)', models: ['claude-sonnet-4-20250514', 'claude-3-5-haiku-20241007'] },
  { value: 'custom', label: 'OpenAI Compatible', models: ['gpt-4o', 'gpt-4o-mini'] },
]

const TEST_QUESTIONS = [
  '¿Qué archivos están relacionados con la autenticación?',
  'Explicá la arquitectura general del proyecto.',
  '¿Hay patrones de diseño reconocibles?',
]

export function ApiKeySetup({ onConfigured, onSkip }: ApiKeySetupProps) {
  const [provider, setProvider] = useState<'anthropic' | 'custom'>('anthropic')
  const [apiKey, setApiKey] = useState('')
  const [model, setModel] = useState(PROVIDER_OPTIONS[0].models[0])
  const [endpoint, setEndpoint] = useState('')
  const [status, setStatus] = useState<'idle' | 'saving' | 'success' | 'error'>('idle')
  const [errorMsg, setErrorMsg] = useState('')

  const handleProviderChange = (p: 'anthropic' | 'custom') => {
    setProvider(p)
    setModel(PROVIDER_OPTIONS.find((o) => o.value === p)?.models[0] || '')
  }

  const handleSave = async () => {
    const trimmedKey = apiKey.trim()
    if (!trimmedKey) {
      setErrorMsg('La API key no puede estar vacía.')
      setStatus('error')
      return
    }

    setStatus('saving')
    setErrorMsg('')

    try {
      await configureAI({
        provider,
        api_key: trimmedKey,
        model,
        endpoint: provider === 'custom' ? endpoint : undefined,
      })
      setStatus('success')
      onConfigured?.()
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Error al guardar la configuración.'
      setErrorMsg(msg)
      setStatus('error')
    }
  }

  return (
    <div className="flex flex-col items-center justify-center min-h-full px-8 py-12 bg-gradient-to-b from-slate-50 to-white">
      {/* Icon */}
      <div className="text-6xl mb-6">🔑</div>

      {/* Title */}
      <h1 className="text-2xl font-bold text-gray-900 mb-2 text-center">
        Configurá tu API Key de IA
      </h1>
      <p className="text-sm text-gray-500 text-center max-w-sm mb-8">
        CodeAtlas usa IA para explicar código y responder preguntas. Tu key se guarda
        de forma segura en el sistema, nunca se comparte.
      </p>

      {/* Form */}
      <div className="w-full max-w-md space-y-5">
        {/* Provider selector */}
        <div>
          <label className="block text-xs font-semibold text-gray-500 uppercase tracking-wide mb-1.5">
            Proveedor
          </label>
          <div className="flex gap-3">
            {PROVIDER_OPTIONS.map((opt) => (
              <button
                key={opt.value}
                onClick={() => handleProviderChange(opt.value as 'anthropic' | 'custom')}
                className={`flex-1 px-4 py-2.5 rounded-lg border text-sm font-medium transition-all ${
                  provider === opt.value
                    ? 'border-blue-500 bg-blue-50 text-blue-700 shadow-sm'
                    : 'border-gray-200 bg-white text-gray-600 hover:bg-gray-50'
                }`}
              >
                {opt.label}
              </button>
            ))}
          </div>
        </div>

        {/* API key input */}
        <div>
          <label className="block text-xs font-semibold text-gray-500 uppercase tracking-wide mb-1.5">
            API Key
          </label>
          <input
            type="password"
            value={apiKey}
            onChange={(e) => setApiKey(e.target.value)}
            placeholder="sk-ant-..."
            className="w-full px-4 py-2.5 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent placeholder:text-gray-400"
          />
          <p className="text-xs text-gray-400 mt-1">
            {provider === 'anthropic'
              ? 'Obtené tu key en console.anthropic.com'
              : 'Obtené tu key del proveedor compatible con OpenAI'}
          </p>
        </div>

        {/* Model selector */}
        <div>
          <label className="block text-xs font-semibold text-gray-500 uppercase tracking-wide mb-1.5">
            Modelo
          </label>
          <select
            value={model}
            onChange={(e) => setModel(e.target.value)}
            className="w-full px-4 py-2.5 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent bg-white"
          >
            {PROVIDER_OPTIONS.find((o) => o.value === provider)?.models.map((m) => (
              <option key={m} value={m}>{m}</option>
            ))}
          </select>
        </div>

        {/* Custom endpoint (only if custom) */}
        {provider === 'custom' && (
          <div>
            <label className="block text-xs font-semibold text-gray-500 uppercase tracking-wide mb-1.5">
              Endpoint
            </label>
            <input
              type="url"
              value={endpoint}
              onChange={(e) => setEndpoint(e.target.value)}
              placeholder="https://api.openai.com/v1"
              className="w-full px-4 py-2.5 border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent placeholder:text-gray-400"
            />
          </div>
        )}

        {/* Status messages */}
        {status === 'error' && (
          <div className="flex items-center gap-2 text-red-600 text-sm bg-red-50 border border-red-100 rounded-lg px-4 py-3">
            <span>⚠️</span>
            <span>{errorMsg}</span>
          </div>
        )}

        {status === 'success' && (
          <div className="flex items-center gap-2 text-green-700 text-sm bg-green-50 border border-green-100 rounded-lg px-4 py-3">
            <span>✅</span>
            <span>Configuración guardada. ¡Listo para usar!</span>
          </div>
        )}

        {/* Actions */}
        <div className="flex gap-3 pt-2">
          <button
            onClick={handleSave}
            disabled={status === 'saving'}
            className="flex-1 px-4 py-2.5 bg-blue-600 text-white text-sm font-semibold rounded-lg hover:bg-blue-700 active:bg-blue-800 transition-colors disabled:opacity-60 disabled:cursor-not-allowed"
          >
            {status === 'saving' ? 'Guardando...' : 'Guardar y continuar'}
          </button>
          {onSkip && (
            <button
              onClick={onSkip}
              className="px-4 py-2.5 text-sm text-gray-500 hover:text-gray-700 transition-colors"
            >
              Omitir por ahora
            </button>
          )}
        </div>

        {/* Test questions */}
        <div className="border-t border-gray-100 pt-5">
          <p className="text-xs text-gray-400 mb-2">Ejemplos de preguntas que podés hacer:</p>
          <div className="flex flex-wrap gap-1">
            {TEST_QUESTIONS.map((q) => (
              <span
                key={q}
                className="px-2 py-1 text-xs bg-gray-100 text-gray-600 rounded border border-gray-200"
              >
                {q}
              </span>
            ))}
          </div>
        </div>
      </div>
    </div>
  )
}