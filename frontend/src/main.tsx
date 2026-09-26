import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App'
// Bundled rather than loaded from Google Fonts: the app promises that nothing
// leaves the device, and a font request on every start would say otherwise.
import '@fontsource/jetbrains-mono/400.css'
import '@fontsource/jetbrains-mono/500.css'
import './index.css'

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
)
