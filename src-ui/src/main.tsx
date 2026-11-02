import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './index.css'
import App from './App.tsx'
import { AppProvider } from './context/AppContext'
import { ConversionStoreProvider } from './context/ConversionStore'
import { ConfirmProvider } from './components/ui/confirm'
import { BatchProvider } from './context/BatchStore'

// `?mock` serves the UI from fixtures so panes can be previewed without a
// Tauri build. Dev only — the import is tree-shaken out of release bundles.
if (import.meta.env.DEV && new URLSearchParams(window.location.search).has('mock')) {
  const { installMockBackend } = await import('./dev/mockBackend');
  installMockBackend();
}

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <AppProvider>
      <ConversionStoreProvider>
        <BatchProvider>
          <ConfirmProvider>
            <App />
          </ConfirmProvider>
        </BatchProvider>
      </ConversionStoreProvider>
    </AppProvider>
  </StrictMode>,
)
