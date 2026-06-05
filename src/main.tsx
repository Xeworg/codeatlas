import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App'
import './styles/index.css'

// Runtime error handlers — crash in React tree (including OutlineView)
// shows a readable error page instead of a silent black screen.
function showError(error: unknown) {
  const root = document.getElementById('root')
  if (!root) return
  const msg = error instanceof Error ? error.message : String(error)
  const el = document.createElement('div')
  el.style.cssText =
    'color:#f87171;padding:24px;font-family:monospace;background:#0a0a0b;min-height:100vh'
  const h1 = document.createElement('h1')
  h1.textContent = 'CodeAtlas — render error'
  h1.style.cssText = 'margin:0 0 12px;font-size:18px'
  const p = document.createElement('p')
  p.textContent = 'React crashed after mounting. Check the Tauri devtools console (Ctrl+Shift+I).'
  p.style.cssText = 'color:#e5e7eb;margin:0 0 12px'
  const pre = document.createElement('pre')
  pre.textContent = msg
  pre.style.cssText = 'color:#f87171;overflow:auto'
  el.append(h1, p, pre)
  root.replaceChildren(el)
}
window.addEventListener('error', (e) => showError(e.error ?? e.message))
window.addEventListener('unhandledrejection', (e) => showError(e.reason))

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
)
