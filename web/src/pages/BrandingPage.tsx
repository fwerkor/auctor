import { AddRounded, DeleteOutlineRounded, OpenInNewRounded, PaletteRounded, ShieldOutlined } from '@mui/icons-material'
import {
  Alert,
  Box,
  Button,
  Card,
  CardContent,
  Stack,
  IconButton,
  TextField,
  Typography,
} from '@mui/material'
import { useEffect, useState } from 'react'
import { api } from '../api'
import { BrandMark, useBrand } from '../components/Branding'
import type { Branding, ReservedUsername } from '../types'

export function BrandingPage() {
  const { brand, refresh } = useBrand()
  const [draft, setDraft] = useState<Branding>(brand)
  const [saved, setSaved] = useState(false)
  const [error, setError] = useState(false)
  const [busy, setBusy] = useState(false)
  const [reserved, setReserved] = useState<ReservedUsername[]>([])
  const [newUsername, setNewUsername] = useState('')
  const [newNote, setNewNote] = useState('')

  useEffect(() => setDraft(brand), [brand])

  async function loadReserved() {
    const result = await api.reservedUsernames()
    setReserved(result.items)
  }

  useEffect(() => {
    loadReserved()
  }, [])

  async function save() {
    setBusy(true)
    setError(false)
    setSaved(false)
    try {
      await api.updateBranding({
        site_name: draft.site_name.trim(),
        site_url: draft.site_url.trim(),
        logo_url: draft.logo_url.trim() || '/brand/auctor.svg',
      })
      await refresh()
      setSaved(true)
    } catch {
      setError(true)
    } finally {
      setBusy(false)
    }
  }

  return (
    <Stack spacing={2.5}>
      <Box>
        <Typography variant="h4">Settings</Typography>
        <Typography color="text.secondary" sx={{ mt: 0.5 }}>
          Configure public branding and account naming policy.
        </Typography>
      </Box>

      {saved && <Alert severity="success">Branding updated.</Alert>}
      {error && (
        <Alert severity="error">
          Could not save these values. Use an HTTPS public URL and an HTTPS or local logo URL.
        </Alert>
      )}

      <Box>
        <Typography variant="h6">Branding</Typography>
        <Typography variant="body2" color="text.secondary" sx={{ mt: 0.35 }}>
          Site identity shown to users and OAuth integrations.
        </Typography>
      </Box>

      <Card sx={{ borderRadius: 2 }}>
        <CardContent sx={{ p: { xs: 2.5, sm: 3.5 } }}>
          <Stack direction={{ xs: 'column', md: 'row' }} spacing={4}>
            <Stack spacing={2.25} sx={{ flex: 1 }}>
              <TextField
                label="Site name"
                value={draft.site_name}
                onChange={(event) => setDraft({ ...draft, site_name: event.target.value })}
                helperText="Shown on sign-in, navigation, and the browser title."
              />
              <TextField
                label="Public site URL"
                value={draft.site_url}
                onChange={(event) => setDraft({ ...draft, site_url: event.target.value })}
                placeholder="https://account.example.com"
                helperText="Canonical public origin used by OAuth metadata and integrations."
              />
              <TextField
                label="Logo URL"
                value={draft.logo_url}
                onChange={(event) => setDraft({ ...draft, logo_url: event.target.value })}
                placeholder="/brand/auctor.svg"
                helperText="Use a local path or an HTTPS image URL. SVG is recommended."
              />
              <Button
                variant="contained"
                onClick={save}
                disabled={busy || !draft.site_name.trim()}
                sx={{ alignSelf: 'flex-end' }}
              >
                Save branding
              </Button>
            </Stack>

            <Box
              sx={{
                width: { xs: '100%', md: 280 },
                minHeight: 220,
                border: '1px solid',
                borderColor: 'divider',
                borderRadius: 2,
                display: 'grid',
                placeItems: 'center',
                bgcolor: '#f8fafd',
                p: 3,
              }}
            >
              <Stack alignItems="center" spacing={1.5}>
                <BrandMark size={72} logoUrl={draft.logo_url || '/brand/auctor.svg'} />
                <Typography variant="h5" textAlign="center">
                  {draft.site_name || 'Your identity service'}
                </Typography>
                {draft.site_url && (
                  <Button
                    size="small"
                    endIcon={<OpenInNewRounded />}
                    href={draft.site_url}
                    target="_blank"
                    rel="noreferrer"
                  >
                    Open public site
                  </Button>
                )}
                <PaletteRounded color="action" sx={{ mt: 1 }} />
              </Stack>
            </Box>
          </Stack>
        </CardContent>
      </Card>

      <Box sx={{ pt: 1 }}>
        <Typography variant="h6">Reserved usernames</Typography>
        <Typography variant="body2" color="text.secondary" sx={{ mt: 0.35 }}>
          Exact, case-insensitive names that cannot be assigned to new users or used in a rename.
        </Typography>
      </Box>

      <Card sx={{ borderRadius: 2 }}>
        <CardContent sx={{ p: { xs: 2.5, sm: 3.5 } }}>
          <Stack spacing={2.5}>
            <Stack direction={{ xs: 'column', md: 'row' }} spacing={1.5}>
              <TextField
                label="Username"
                value={newUsername}
                onChange={(event) => setNewUsername(event.target.value)}
                placeholder="example"
                sx={{ flex: 1 }}
              />
              <TextField
                label="Reason"
                value={newNote}
                onChange={(event) => setNewNote(event.target.value)}
                placeholder="Reserved platform name"
                sx={{ flex: 2 }}
              />
              <Button
                variant="contained"
                startIcon={<AddRounded />}
                disabled={!newUsername.trim()}
                onClick={async () => {
                  await api.addReservedUsername(newUsername, newNote)
                  setNewUsername('')
                  setNewNote('')
                  await loadReserved()
                }}
              >
                Add
              </Button>
            </Stack>

            <Stack spacing={0.75}>
              {reserved.map((item) => (
                <Stack
                  key={item.username}
                  direction="row"
                  alignItems="center"
                  spacing={1.5}
                  sx={{
                    px: 1.5,
                    py: 1.1,
                    border: '1px solid',
                    borderColor: 'divider',
                    borderRadius: 1.5,
                  }}
                >
                  <ShieldOutlined fontSize="small" color="action" />
                  <Box sx={{ flex: 1, minWidth: 0 }}>
                    <Typography fontWeight={600} sx={{ fontFamily: 'monospace' }}>
                      {item.username}
                    </Typography>
                    {item.note && (
                      <Typography variant="caption" color="text.secondary">
                        {item.note}
                      </Typography>
                    )}
                  </Box>
                  <IconButton
                    size="small"
                    aria-label={'Remove reserved username ' + item.username}
                    onClick={async () => {
                      await api.deleteReservedUsername(item.username)
                      await loadReserved()
                    }}
                  >
                    <DeleteOutlineRounded fontSize="small" />
                  </IconButton>
                </Stack>
              ))}
            </Stack>
          </Stack>
        </CardContent>
      </Card>
    </Stack>
  )
}
