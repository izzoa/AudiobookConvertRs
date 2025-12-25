import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './index.css'
import App from './App.tsx'
import { AppProvider } from './context/AppContext'

console.log("main.tsx: Starting React app...");
const rootElement = document.getElementById('root');
console.log("main.tsx: Root element found:", !!rootElement);

if (rootElement) {
  try {
    console.log("main.tsx: Creating React root...");
    const root = createRoot(rootElement);
    console.log("main.tsx: Rendering app...");
    root.render(
      <StrictMode>
        <AppProvider>
          <App />
        </AppProvider>
      </StrictMode>,
    );
    console.log("main.tsx: Render call complete");
  } catch (error) {
    console.error("main.tsx: Error during React render:", error);
  }
}
