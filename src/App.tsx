import { useState } from 'react'

function App() {
  const [status] = useState('idle')
  return (
    <div className="flex h-screen flex-col bg-surface-base text-white">
      <header className="flex items-center gap-4 border-b border-white/10 px-4 py-2">
        <span className="text-sm font-medium">CodeAtlas</span>
        <span className="text-xs text-white/40">{status}</span>
      </header>
      <main className="flex flex-1 items-center justify-center text-white/30">
        <p>Seleccioná un proyecto para comenzar</p>
      </main>
    </div>
  )
}

export default App
