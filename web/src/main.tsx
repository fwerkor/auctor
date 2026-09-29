import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { BrowserRouter } from 'react-router-dom'
import App from './App'
import { AppThemeProvider } from './colorMode'
import { BrandProvider } from './components/Branding'

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <AppThemeProvider>
      <BrandProvider>
        <BrowserRouter>
          <App />
        </BrowserRouter>
      </BrandProvider>
    </AppThemeProvider>
  </StrictMode>,
)
