import { createTheme } from '@mui/material/styles'
import type { PaletteMode } from '@mui/material'

export function createAppTheme(mode: PaletteMode) {
  const dark = mode === 'dark'

  return createTheme({
    palette: {
      mode,
      primary: {
        main: dark ? '#8ab4f8' : '#0b57d0',
        light: dark ? '#203553' : '#e8f0fe',
        dark: dark ? '#aecbfa' : '#0842a0',
        contrastText: dark ? '#10233f' : '#ffffff',
      },
      background: {
        default: dark ? '#111318' : '#f8fafd',
        paper: dark ? '#1b1f24' : '#ffffff',
      },
      text: {
        primary: dark ? '#e8eaed' : '#1f1f1f',
        secondary: dark ? '#9aa0a6' : '#5f6368',
      },
      divider: dark ? '#343a40' : '#e3e7ec',
      action: {
        hover: dark ? 'rgba(255,255,255,.06)' : 'rgba(60,64,67,.06)',
        selected: dark ? 'rgba(138,180,248,.14)' : 'rgba(11,87,208,.09)',
      },
    },
    shape: {
      borderRadius: 8,
    },
    typography: {
      fontFamily: '"Google Sans", "Roboto", "Helvetica Neue", Arial, sans-serif',
      h4: { fontWeight: 500, letterSpacing: '-0.02em' },
      h5: { fontWeight: 500, letterSpacing: '-0.015em' },
      h6: { fontWeight: 500 },
      button: { textTransform: 'none', fontWeight: 600 },
    },
    components: {
      MuiButton: {
        styleOverrides: {
          root: { borderRadius: 8, paddingInline: 18, minHeight: 40 },
        },
      },
      MuiCard: {
        styleOverrides: {
          root: ({ theme }) => ({
            boxShadow: 'none',
            border: '1px solid',
            borderColor: theme.palette.divider,
          }),
        },
      },
      MuiOutlinedInput: {
        styleOverrides: {
          root: { borderRadius: 10 },
        },
      },
      MuiChip: {
        styleOverrides: {
          root: { borderRadius: 8, fontWeight: 500 },
        },
      },
      MuiCssBaseline: {
        styleOverrides: {
          'html, body, #root': {
            minHeight: '100%',
          },
          body: {
            transition: 'background-color 120ms ease, color 120ms ease',
          },
        },
      },
    },
  })
}
