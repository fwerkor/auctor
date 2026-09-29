import {
  AddRounded,
  ContentCopyRounded,
  DeleteOutlineRounded,
  DnsRounded,
  EditRounded,
  LanguageRounded,
  PhoneIphoneRounded,
} from '@mui/icons-material'
import {
  Box,
  Button,
  Card,
  CardContent,
  Chip,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  FormControl,
  IconButton,
  InputLabel,
  MenuItem,
  Select,
  Stack,
  Switch,
  TextField,
  Tooltip,
  Typography,
} from '@mui/material'
import { FormEvent, useCallback, useEffect, useState } from 'react'
import { api } from '../api'
import type { Application } from '../types'

type Draft = {
  id?: string
  name: string
  client_id: string
  app_type: 'web' | 'native' | 'service'
  redirect_uris: string
  status: 'active' | 'disabled'
}

const emptyDraft: Draft = {
  name: '',
  client_id: '',
  app_type: 'web',
  redirect_uris: '',
  status: 'active',
}

function iconFor(type: Application['app_type']) {
  if (type === 'native') return <PhoneIphoneRounded />
  if (type === 'service') return <DnsRounded />
  return <LanguageRounded />
}

export function ApplicationsPage() {
  const [items, setItems] = useState<Application[]>([])
  const [editor, setEditor] = useState<Draft | null>(null)
  const [deleteTarget, setDeleteTarget] = useState<Application | null>(null)

  const load = useCallback(async () => {
    const result = await api.applications()
    setItems(result.items)
  }, [])

  useEffect(() => {
    load()
  }, [load])

  return (
    <Stack spacing={2.5}>
      <Stack direction={{ xs: 'column', sm: 'row' }} justifyContent="space-between" gap={2}>
        <Box>
          <Typography variant="h4">Applications</Typography>
          <Typography color="text.secondary" sx={{ mt: 0.5 }}>
            Register clients that will use this identity service for OAuth 2.0.
          </Typography>
        </Box>
        <Button
          variant="contained"
          startIcon={<AddRounded />}
          onClick={() => setEditor({ ...emptyDraft })}
        >
          Add application
        </Button>
      </Stack>

      <Stack spacing={1.5}>
        {items.map((application) => (
          <Card key={application.id} sx={{ borderRadius: 1.5 }}>
            <CardContent sx={{ py: 2.5, '&:last-child': { pb: 2.5 } }}>
              <Stack direction="row" spacing={2} alignItems="flex-start">
                <Box
                  sx={{
                    width: 48,
                    height: 48,
                    borderRadius: 1.5,
                    bgcolor: 'action.hover',
                    color: 'primary.main',
                    display: 'grid',
                    placeItems: 'center',
                    flex: '0 0 auto',
                  }}
                >
                  {iconFor(application.app_type)}
                </Box>

                <Box sx={{ minWidth: 0, flex: 1 }}>
                  <Stack direction="row" spacing={1} alignItems="center" flexWrap="wrap">
                    <Typography variant="h6">{application.name}</Typography>
                    <Chip
                      size="small"
                      label={application.app_type}
                      variant="outlined"
                    />
                    <Chip
                      size="small"
                      label={application.status === 'active' ? 'Active' : 'Disabled'}
                      color={application.status === 'active' ? 'success' : 'default'}
                      variant="outlined"
                    />
                  </Stack>

                  <Stack direction="row" spacing={0.5} alignItems="center" sx={{ mt: 0.6 }}>
                    <Typography
                      variant="body2"
                      color="text.secondary"
                      sx={{ fontFamily: 'monospace', overflowWrap: 'anywhere' }}
                    >
                      {application.client_id}
                    </Typography>
                    <Tooltip title="Copy client ID">
                      <IconButton
                        size="small"
                        onClick={() => navigator.clipboard.writeText(application.client_id)}
                      >
                        <ContentCopyRounded sx={{ fontSize: 16 }} />
                      </IconButton>
                    </Tooltip>
                  </Stack>

                  <Typography variant="body2" color="text.secondary" sx={{ mt: 0.75 }}>
                    {application.redirect_uris.length
                      ? application.redirect_uris.length +
                        (application.redirect_uris.length === 1
                          ? ' redirect URI'
                          : ' redirect URIs')
                      : 'No redirect URIs'}
                  </Typography>
                </Box>

                <IconButton
                  aria-label={'Edit ' + application.name}
                  onClick={() =>
                    setEditor({
                      id: application.id,
                      name: application.name,
                      client_id: application.client_id,
                      app_type: application.app_type,
                      redirect_uris: application.redirect_uris.join('\n'),
                      status: application.status,
                    })
                  }
                >
                  <EditRounded fontSize="small" />
                </IconButton>
                <IconButton
                  aria-label={'Delete ' + application.name}
                  onClick={() => setDeleteTarget(application)}
                >
                  <DeleteOutlineRounded fontSize="small" />
                </IconButton>
              </Stack>
            </CardContent>
          </Card>
        ))}

        {!items.length && (
          <Card sx={{ borderRadius: 2 }}>
            <CardContent sx={{ py: 8, textAlign: 'center' }}>
              <LanguageRounded sx={{ fontSize: 52, color: 'primary.main', mb: 2 }} />
              <Typography variant="h6">No applications registered</Typography>
              <Typography color="text.secondary" sx={{ maxWidth: 560, mx: 'auto', mt: 1 }}>
                Add an application to reserve its client ID and redirect URI policy. Protocol
                credentials will build on this same registration.
              </Typography>
              <Button
                variant="contained"
                startIcon={<AddRounded />}
                sx={{ mt: 3 }}
                onClick={() => setEditor({ ...emptyDraft })}
              >
                Add application
              </Button>
            </CardContent>
          </Card>
        )}
      </Stack>

      <ApplicationEditor
        value={editor}
        onClose={() => setEditor(null)}
        onSaved={async () => {
          setEditor(null)
          await load()
        }}
      />

      <Dialog
        open={Boolean(deleteTarget)}
        onClose={() => setDeleteTarget(null)}
        maxWidth="xs"
        fullWidth
      >
        <DialogTitle>Delete application?</DialogTitle>
        <DialogContent>
          <Typography color="text.secondary">
            {deleteTarget?.name} will no longer be registered with Auctor. This action does not
            delete the external application itself.
          </Typography>
        </DialogContent>
        <DialogActions sx={{ p: 2.5 }}>
          <Button onClick={() => setDeleteTarget(null)}>Cancel</Button>
          <Button
            color="error"
            variant="contained"
            onClick={async () => {
              if (!deleteTarget) return
              await api.deleteApplication(deleteTarget.id)
              setDeleteTarget(null)
              await load()
            }}
          >
            Delete
          </Button>
        </DialogActions>
      </Dialog>
    </Stack>
  )
}

function ApplicationEditor({
  value,
  onClose,
  onSaved,
}: {
  value: Draft | null
  onClose: () => void
  onSaved: () => Promise<void>
}) {
  const [draft, setDraft] = useState<Draft | null>(value)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')

  useEffect(() => {
    setDraft(value)
    setError('')
  }, [value])

  if (!draft) return null
  const editing = Boolean(draft.id)

  async function submit(event: FormEvent) {
    event.preventDefault()
    if (!draft) return
    setBusy(true)
    setError('')
    const redirectUris = draft.redirect_uris
      .split('\n')
      .map((value) => value.trim())
      .filter(Boolean)

    try {
      if (draft.id) {
        await api.updateApplication(draft.id, {
          name: draft.name,
          client_id: draft.client_id,
          app_type: draft.app_type,
          redirect_uris: redirectUris,
          status: draft.status,
        })
      } else {
        await api.createApplication({
          name: draft.name,
          client_id: draft.client_id || undefined,
          app_type: draft.app_type,
          redirect_uris: redirectUris,
        })
      }
      await onSaved()
    } catch {
      setError(
        'Could not save this application. Check the client ID and redirect URIs for conflicts or invalid values.',
      )
    } finally {
      setBusy(false)
    }
  }

  return (
    <Dialog open={Boolean(value)} onClose={onClose} maxWidth="sm" fullWidth>
      <Box component="form" onSubmit={submit}>
        <DialogTitle>{editing ? 'Edit application' : 'Add application'}</DialogTitle>
        <DialogContent>
          <Stack spacing={2.25} sx={{ pt: 1 }}>
            {error && (
              <Typography color="error" variant="body2">
                {error}
              </Typography>
            )}

            <TextField
              label="Application name"
              value={draft.name}
              onChange={(event) => setDraft({ ...draft, name: event.target.value })}
              required
              autoFocus
            />

            <FormControl>
              <InputLabel>Application type</InputLabel>
              <Select
                value={draft.app_type}
                label="Application type"
                onChange={(event) =>
                  setDraft({
                    ...draft,
                    app_type: event.target.value as Draft['app_type'],
                  })
                }
              >
                <MenuItem value="web">Web application</MenuItem>
                <MenuItem value="native">Native / CLI application</MenuItem>
                <MenuItem value="service">Service / machine client</MenuItem>
              </Select>
            </FormControl>

            <TextField
              label="Client ID"
              value={draft.client_id}
              onChange={(event) => setDraft({ ...draft, client_id: event.target.value })}
              helperText={
                editing
                  ? 'Changing a client ID can break existing integrations.'
                  : 'Leave empty to let Auctor generate a stable client ID.'
              }
            />

            <TextField
              label="Redirect URIs"
              value={draft.redirect_uris}
              onChange={(event) => setDraft({ ...draft, redirect_uris: event.target.value })}
              multiline
              minRows={4}
              placeholder={'https://example.com/oauth/callback\nhttp://localhost:3000/callback'}
              helperText="One exact URI per line. HTTPS is required except localhost loopback callbacks."
            />

            {editing && (
              <Stack direction="row" alignItems="center" justifyContent="space-between">
                <Box>
                  <Typography fontWeight={600}>Application enabled</Typography>
                  <Typography variant="body2" color="text.secondary">
                    Disabled applications will be rejected when protocol endpoints are enabled.
                  </Typography>
                </Box>
                <Switch
                  checked={draft.status === 'active'}
                  onChange={(event) =>
                    setDraft({
                      ...draft,
                      status: event.target.checked ? 'active' : 'disabled',
                    })
                  }
                />
              </Stack>
            )}
          </Stack>
        </DialogContent>
        <DialogActions sx={{ p: 2.5 }}>
          <Button onClick={onClose}>Cancel</Button>
          <Button type="submit" variant="contained" disabled={busy || !draft.name.trim()}>
            {editing ? 'Save changes' : 'Add application'}
          </Button>
        </DialogActions>
      </Box>
    </Dialog>
  )
}
