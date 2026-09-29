import {
  Alert,
  Box,
  Button,
  Card,
  CardContent,
  CircularProgress,
  Stack,
  TextField,
  Typography,
} from '@mui/material'
import { FormEvent, useState } from 'react'
import { api } from '../api'

type Props = {
  onAuthenticated: () => Promise<void>
}

export function LoginPage({ onAuthenticated }: Props) {
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState(false)

  async function submit(event: FormEvent) {
    event.preventDefault()
    setBusy(true)
    setError(false)
    try {
      await api.login(username, password)
      await onAuthenticated()
    } catch {
      setError(true)
    } finally {
      setBusy(false)
    }
  }

  return (
    <Box
      sx={{
        minHeight: '100vh',
        display: 'grid',
        placeItems: 'center',
        px: 2,
        background:
          'radial-gradient(circle at 20% 10%, rgba(211,227,253,.75), transparent 32%), #f8fafd',
      }}
    >
      <Card sx={{ width: '100%', maxWidth: 460, borderRadius: 7 }}>
        <CardContent sx={{ p: { xs: 3.5, sm: 5 } }}>
          <Stack spacing={3}>
            <Box>
              <Box
                sx={{
                  width: 48,
                  height: 48,
                  borderRadius: 3,
                  bgcolor: 'primary.main',
                  color: 'white',
                  display: 'grid',
                  placeItems: 'center',
                  fontSize: 26,
                  fontWeight: 600,
                  mb: 3,
                }}
              >
                A
              </Box>
              <Typography variant="h4">Sign in to Auctor</Typography>
              <Typography color="text.secondary" sx={{ mt: 1 }}>
                One account for your applications and infrastructure.
              </Typography>
            </Box>
            {error && <Alert severity="error">Incorrect username, email, or password.</Alert>}
            <Box component="form" onSubmit={submit}>
              <Stack spacing={2.25}>
                <TextField
                  label="Username or email"
                  value={username}
                  onChange={(e) => setUsername(e.target.value)}
                  autoComplete="username"
                  autoFocus
                  fullWidth
                />
                <TextField
                  label="Password"
                  type="password"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  autoComplete="current-password"
                  fullWidth
                />
                <Button
                  type="submit"
                  variant="contained"
                  size="large"
                  disabled={busy || !username || !password}
                  sx={{ alignSelf: 'flex-end', minWidth: 112 }}
                >
                  {busy ? <CircularProgress size={20} color="inherit" /> : 'Sign in'}
                </Button>
              </Stack>
            </Box>
          </Stack>
        </CardContent>
      </Card>
    </Box>
  )
}
