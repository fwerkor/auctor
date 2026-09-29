import {
  BrightnessAutoRounded,
  DarkModeRounded,
  LightModeRounded,
} from '@mui/icons-material'
import {
  CssBaseline,
  IconButton,
  ListItemIcon,
  ListItemText,
  Menu,
  MenuItem,
  ThemeProvider,
  Tooltip,
} from '@mui/material'
import { createContext, useContext, useEffect, useMemo, useState } from 'react'
import type { MouseEvent, ReactNode } from 'react'
import { createAppTheme } from './theme'

export type ThemePreference = 'auto' | 'light' | 'dark'

const STORAGE_KEY = 'auctor-color-mode'

const ColorModeContext = createContext<{
  preference: ThemePreference
  resolved: 'light' | 'dark'
  setPreference: (value: ThemePreference) => void
}>({
  preference: 'auto',
  resolved: 'light',
  setPreference: () => {},
})

function readPreference(): ThemePreference {
  try {
    const value = localStorage.getItem(STORAGE_KEY)
    if (value === 'light' || value === 'dark' || value === 'auto') return value
  } catch {
    // Ignore storage access failures.
  }
  return 'auto'
}

function prefersDark() {
  return typeof window !== 'undefined' && window.matchMedia('(prefers-color-scheme: dark)').matches
}

export function AppThemeProvider({ children }: { children: ReactNode }) {
  const [preference, setPreferenceState] = useState<ThemePreference>(readPreference)
  const [systemDark, setSystemDark] = useState(prefersDark)

  useEffect(() => {
    const media = window.matchMedia('(prefers-color-scheme: dark)')
    const onChange = (event: MediaQueryListEvent) => setSystemDark(event.matches)
    media.addEventListener('change', onChange)
    return () => media.removeEventListener('change', onChange)
  }, [])

  const resolved: 'light' | 'dark' =
    preference === 'auto' ? (systemDark ? 'dark' : 'light') : preference

  const setPreference = (value: ThemePreference) => {
    setPreferenceState(value)
    try {
      localStorage.setItem(STORAGE_KEY, value)
    } catch {
      // The in-memory preference still applies for this page.
    }
  }

  useEffect(() => {
    document.documentElement.dataset.theme = resolved
    document.documentElement.style.colorScheme = resolved
  }, [resolved])

  const theme = useMemo(() => createAppTheme(resolved), [resolved])
  const value = useMemo(
    () => ({ preference, resolved, setPreference }),
    [preference, resolved],
  )

  return (
    <ColorModeContext.Provider value={value}>
      <ThemeProvider theme={theme}>
        <CssBaseline />
        {children}
      </ThemeProvider>
    </ColorModeContext.Provider>
  )
}

export function useColorMode() {
  return useContext(ColorModeContext)
}

export function ThemeModeButton() {
  const { preference, setPreference } = useColorMode()
  const [anchor, setAnchor] = useState<HTMLElement | null>(null)

  const Icon =
    preference === 'auto'
      ? BrightnessAutoRounded
      : preference === 'dark'
        ? DarkModeRounded
        : LightModeRounded

  const openMenu = (event: MouseEvent<HTMLElement>) => setAnchor(event.currentTarget)
  const choose = (value: ThemePreference) => {
    setPreference(value)
    setAnchor(null)
  }

  return (
    <>
      <Tooltip title={'Appearance: ' + preference[0].toUpperCase() + preference.slice(1)}>
        <IconButton
          color="inherit"
          aria-label="Change appearance"
          aria-haspopup="menu"
          aria-expanded={Boolean(anchor)}
          onClick={openMenu}
        >
          <Icon fontSize="small" />
        </IconButton>
      </Tooltip>
      <Menu anchorEl={anchor} open={Boolean(anchor)} onClose={() => setAnchor(null)}>
        <MenuItem selected={preference === 'auto'} onClick={() => choose('auto')}>
          <ListItemIcon><BrightnessAutoRounded fontSize="small" /></ListItemIcon>
          <ListItemText>Auto</ListItemText>
        </MenuItem>
        <MenuItem selected={preference === 'light'} onClick={() => choose('light')}>
          <ListItemIcon><LightModeRounded fontSize="small" /></ListItemIcon>
          <ListItemText>Light</ListItemText>
        </MenuItem>
        <MenuItem selected={preference === 'dark'} onClick={() => choose('dark')}>
          <ListItemIcon><DarkModeRounded fontSize="small" /></ListItemIcon>
          <ListItemText>Dark</ListItemText>
        </MenuItem>
      </Menu>
    </>
  )
}
