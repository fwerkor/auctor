import {
  Alert,
  Box,
  Button,
  Card,
  CardContent,
  Stack,
  TextField,
  Typography,
} from '@mui/material'
import { FormEvent, useState } from 'react'
import { api } from '../api'
import { BrandMark, useBrand } from '../components/Branding'

type Props = {
  onComplete: () => Promise<void>
}

export function SetupPage({ onComplete }: Props) {
  const { brand } = useBrand()
  const [form, setForm] = useState({
    setup_token: '',
    username: '',
    display_name: '',
    email: '',
    password: '',
  })
  const [error, setError] = useState('')
  const [busy, setBusy] = useState(false)

  async function submit(event: FormEvent) {
    event.preventDefault()
    setBusy(true)
    setError('')
    try {
      await api.setup(form)
      await onComplete()
    } catch (err) {
      const status = (err as { status?: number }).status
      setError(
        status === 401
          ? 'The setup token is not valid.'
          : status === 400
            ? 'One of the account fields is invalid. Check the username, email, and password, then try again.'
            : 'Could not initialize the identity service.',
      )
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
        py: 4,
        background:
          'radial-gradient(circle at 18% 8%, rgba(211,227,253,.9), transparent 30%), #f8fafd',
      }}
    >
      <Card sx={{ width: '100%', maxWidth: 620, borderRadius: 2.5 }}>
        <CardContent sx={{ p: { xs: 3.5, sm: 5 } }}>
          <Stack component="form" spacing={2.25} onSubmit={submit}>
            <Box sx={{ mb: 1 }}>
              <BrandMark size={52} />
            </Box>
            <Box>
              <Typography variant="h4">Set up {brand.site_name}</Typography>
              <Typography color="text.secondary" sx={{ mt: 1 }}>
                Create the first platform administrator. The one-time setup token is stored only
                on the server and is deleted after initialization.
              </Typography>
            </Box>

            {error && <Alert severity="error">{error}</Alert>}

            <TextField
              label="One-time setup token"
              value={form.setup_token}
              onChange={(event) => setForm({ ...form, setup_token: event.target.value })}
              autoComplete="off"
              autoFocus
              required
            />

            <Stack direction={{ xs: 'column', sm: 'row' }} spacing={2}>
              <TextField
                fullWidth
                label="Username"
                value={form.username}
                onChange={(event) => setForm({ ...form, username: event.target.value })}
                autoComplete="username"
                required
              />
              <TextField
                fullWidth
                label="Display name"
                value={form.display_name}
                onChange={(event) => setForm({ ...form, display_name: event.target.value })}
                required
              />
            </Stack>

            <TextField
              label="Email"
              type="email"
              value={form.email}
              onChange={(event) => setForm({ ...form, email: event.target.value })}
              autoComplete="email"
              required
            />

            <TextField
              label="Password"
              type="password"
              value={form.password}
              onChange={(event) => setForm({ ...form, password: event.target.value })}
              autoComplete="new-password"
              helperText="Use at least 12 characters."
              required
            />

            <Button
              type="submit"
              variant="contained"
              size="large"
              disabled={
                busy ||
                form.password.length < 12 ||
                !form.setup_token ||
                !form.username ||
                !form.email ||
                !form.display_name
              }
              sx={{ alignSelf: 'flex-end' }}
            >
              Create administrator
            </Button>
          </Stack>
        </CardContent>
      </Card>
    </Box>
  )
}
