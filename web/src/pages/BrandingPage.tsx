import { OpenInNewRounded, PaletteRounded } from '@mui/icons-material'
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
import { useEffect, useState } from 'react'
import { api } from '../api'
import { BrandMark, useBrand } from '../components/Branding'
import type { Branding } from '../types'

export function BrandingPage() {
  const { brand, refresh } = useBrand()
  const [draft, setDraft] = useState<Branding>(brand)
  const [saved, setSaved] = useState(false)
  const [error, setError] = useState(false)
  const [busy, setBusy] = useState(false)

  useEffect(() => setDraft(brand), [brand])

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
        <Typography variant="h4">Branding</Typography>
        <Typography color="text.secondary" sx={{ mt: 0.5 }}>
          Customize the identity service without rebuilding the project.
        </Typography>
      </Box>

      {saved && <Alert severity="success">Branding updated.</Alert>}
      {error && (
        <Alert severity="error">
          Could not save these values. Use an HTTPS public URL and an HTTPS or local logo URL.
        </Alert>
      )}

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
    </Stack>
  )
}
