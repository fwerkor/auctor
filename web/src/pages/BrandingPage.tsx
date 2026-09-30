import { AddRounded, DeleteOutlineRounded, OpenInNewRounded, PaletteRounded, PersonRounded, ShieldOutlined } from '@mui/icons-material'
import {
  Alert,
  Box,
  Button,
  Card,
  CardContent,
  FormControlLabel,
  Stack,
  IconButton,
  MenuItem,
  Switch,
  TextField,
  Typography,
} from '@mui/material'
import { useEffect, useState } from 'react'
import { api } from '../api'
import { BrandMark, useBrand } from '../components/Branding'
import type { AuthSettings, AvatarSettings, Branding, ReservedUsername } from '../types'

export function BrandingPage() {
  const { brand, refresh } = useBrand()
  const [draft, setDraft] = useState<Branding>(brand)
  const [saved, setSaved] = useState(false)
  const [error, setError] = useState(false)
  const [busy, setBusy] = useState(false)
  const [reserved, setReserved] = useState<ReservedUsername[]>([])
  const [avatarDraft, setAvatarDraft] = useState<AvatarSettings>({
    avatar_source_template: brand.avatar_source_template,
    avatar_delivery: brand.avatar_delivery,
  })
  const [avatarSaved, setAvatarSaved] = useState(false)
  const [avatarError, setAvatarError] = useState(false)
  const [avatarBusy, setAvatarBusy] = useState(false)
  const [newUsername, setNewUsername] = useState('')
  const [newNote, setNewNote] = useState('')
  const [authDraft, setAuthDraft] = useState<AuthSettings>({
    registration_enabled: false,
    registration_require_email_verification: false,
    smtp_host: '',
    smtp_port: 587,
    smtp_security: 'starttls',
    smtp_username: '',
    smtp_from_email: '',
    smtp_from_name: 'Auctor',
    smtp_password_configured: false,
  })
  const [smtpPassword, setSmtpPassword] = useState('')
  const [authSaved, setAuthSaved] = useState(false)
  const [authError, setAuthError] = useState(false)
  const [authBusy, setAuthBusy] = useState(false)
  const [smtpTested, setSmtpTested] = useState(false)

  useEffect(() => {
    setDraft(brand)
  }, [brand])

  async function loadAvatarSettings() {
    const result = await api.avatarSettings()
    setAvatarDraft(result)
  }

  async function loadReserved() {
    const result = await api.reservedUsernames()
    setReserved(result.items)
  }

  async function loadAuthSettings() {
    const result = await api.authSettings()
    setAuthDraft(result)
    setSmtpPassword('')
  }

  useEffect(() => {
    loadAvatarSettings()
    loadReserved()
    loadAuthSettings()
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

  async function saveAvatarSettings() {
    setAvatarBusy(true)
    setAvatarError(false)
    setAvatarSaved(false)
    try {
      await api.updateAvatarSettings({
        avatar_source_template: avatarDraft.avatar_source_template.trim(),
        avatar_delivery: avatarDraft.avatar_delivery,
      })
      await Promise.all([refresh(), loadAvatarSettings()])
      setAvatarSaved(true)
    } catch {
      setAvatarError(true)
    } finally {
      setAvatarBusy(false)
    }
  }

  async function saveAuthSettings() {
    setAuthBusy(true)
    setAuthError(false)
    setAuthSaved(false)
    setSmtpTested(false)
    try {
      const result = await api.updateAuthSettings({
        ...authDraft,
        smtp_host: authDraft.smtp_host.trim(),
        smtp_username: authDraft.smtp_username.trim(),
        smtp_from_email: authDraft.smtp_from_email.trim(),
        smtp_from_name: authDraft.smtp_from_name.trim(),
        ...(smtpPassword ? { smtp_password: smtpPassword } : {}),
      })
      setAuthDraft(result)
      setSmtpPassword('')
      setAuthSaved(true)
      return true
    } catch {
      setAuthError(true)
      return false
    } finally {
      setAuthBusy(false)
    }
  }

  async function testSmtp() {
    setAuthBusy(true)
    setAuthError(false)
    setSmtpTested(false)
    try {
      await api.testSmtp()
      setSmtpTested(true)
    } catch {
      setAuthError(true)
    } finally {
      setAuthBusy(false)
    }
  }

  return (
    <Stack spacing={2.5}>
      <Box>
        <Typography variant="h4">Settings</Typography>
        <Typography color="text.secondary" sx={{ mt: 0.5 }}>
          Configure public branding, avatar delivery, and account naming policy.
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
                bgcolor: 'background.default',
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
        <Typography variant="h6">Avatars</Typography>
        <Typography variant="body2" color="text.secondary" sx={{ mt: 0.35 }}>
          Choose where user avatars come from and whether browsers contact that source directly.
        </Typography>
      </Box>

      {avatarSaved && <Alert severity="success">Avatar settings updated.</Alert>}
      {avatarError && (
        <Alert severity="error">
          Could not save avatar settings. Use an HTTPS template containing {'{email}'} or {'{email_md5}'}.
        </Alert>
      )}

      <Card sx={{ borderRadius: 2 }}>
        <CardContent sx={{ p: { xs: 2.5, sm: 3.5 } }}>
          <Stack direction={{ xs: 'column', md: 'row' }} spacing={4}>
            <Stack spacing={2.25} sx={{ flex: 1 }}>
              <TextField
                label="Avatar source URL template"
                value={avatarDraft.avatar_source_template}
                onChange={(event) =>
                  setAvatarDraft({ ...avatarDraft, avatar_source_template: event.target.value })
                }
                helperText="HTTPS only. Supported placeholders: {email_md5}, {email}, and {size}."
              />
              <TextField
                select
                label="Delivery"
                value={avatarDraft.avatar_delivery}
                onChange={(event) =>
                  setAvatarDraft({
                    ...avatarDraft,
                    avatar_delivery: event.target.value as 'direct' | 'proxy',
                  })
                }
                helperText={
                  avatarDraft.avatar_delivery === 'direct'
                    ? 'Direct: the browser loads avatars from the configured source.'
                    : 'Proxy: Auctor fetches avatars server-side and the browser only contacts Auctor.'
                }
              >
                <MenuItem value="direct">Direct link</MenuItem>
                <MenuItem value="proxy">Reverse proxy</MenuItem>
              </TextField>
              <Button
                variant="contained"
                onClick={saveAvatarSettings}
                disabled={avatarBusy || !avatarDraft.avatar_source_template.trim()}
                sx={{ alignSelf: 'flex-end' }}
              >
                Save avatar settings
              </Button>
            </Stack>

            <Box
              sx={{
                width: { xs: '100%', md: 280 },
                minHeight: 180,
                border: '1px solid',
                borderColor: 'divider',
                borderRadius: 2,
                display: 'grid',
                placeItems: 'center',
                bgcolor: 'background.default',
                p: 3,
              }}
            >
              <Stack alignItems="center" spacing={1.25} textAlign="center">
                <PersonRounded color="primary" sx={{ fontSize: 48 }} />
                <Typography fontWeight={600}>
                  {avatarDraft.avatar_delivery === 'proxy' ? 'Reverse proxy' : 'Direct link'}
                </Typography>
                <Typography variant="body2" color="text.secondary">
                  {avatarDraft.avatar_delivery === 'proxy'
                    ? 'Avatar requests stay behind this Auctor deployment.'
                    : 'The browser contacts the avatar provider directly.'}
                </Typography>
              </Stack>
            </Box>
          </Stack>
        </CardContent>
      </Card>

      <Box sx={{ pt: 1 }}>
        <Typography variant="h6">Registration and email</Typography>
        <Typography variant="body2" color="text.secondary" sx={{ mt: 0.35 }}>
          Control self-service registration and configure the SMTP transport used for verification codes.
        </Typography>
      </Box>

      {authSaved && <Alert severity="success">Registration and SMTP settings updated.</Alert>}
      {smtpTested && <Alert severity="success">Test email sent to your account email address.</Alert>}
      {authError && (
        <Alert severity="error">
          Could not apply the authentication settings or send the SMTP test message. Check the SMTP values and TLS mode.
        </Alert>
      )}

      <Card sx={{ borderRadius: 2 }}>
        <CardContent sx={{ p: { xs: 2.5, sm: 3.5 } }}>
          <Stack spacing={2.25}>
            <FormControlLabel
              control={
                <Switch
                  checked={authDraft.registration_enabled}
                  onChange={(event) =>
                    setAuthDraft({ ...authDraft, registration_enabled: event.target.checked })
                  }
                />
              }
              label="Allow self-service registration"
            />
            <FormControlLabel
              control={
                <Switch
                  checked={authDraft.registration_require_email_verification}
                  onChange={(event) =>
                    setAuthDraft({
                      ...authDraft,
                      registration_require_email_verification: event.target.checked,
                    })
                  }
                />
              }
              label="Require email verification for registration"
            />

            <Stack direction={{ xs: 'column', md: 'row' }} spacing={2}>
              <TextField
                label="SMTP host"
                value={authDraft.smtp_host}
                onChange={(event) => setAuthDraft({ ...authDraft, smtp_host: event.target.value })}
                sx={{ flex: 2 }}
              />
              <TextField
                label="Port"
                type="number"
                value={authDraft.smtp_port}
                onChange={(event) =>
                  setAuthDraft({ ...authDraft, smtp_port: Number(event.target.value) || 0 })
                }
                sx={{ flex: 1 }}
              />
              <TextField
                select
                label="Security"
                value={authDraft.smtp_security}
                onChange={(event) =>
                  setAuthDraft({
                    ...authDraft,
                    smtp_security: event.target.value as 'starttls' | 'tls' | 'none',
                  })
                }
                sx={{ flex: 1 }}
              >
                <MenuItem value="starttls">STARTTLS</MenuItem>
                <MenuItem value="tls">Implicit TLS</MenuItem>
                <MenuItem value="none">None</MenuItem>
              </TextField>
            </Stack>

            <TextField
              label="SMTP username"
              value={authDraft.smtp_username}
              onChange={(event) =>
                setAuthDraft({ ...authDraft, smtp_username: event.target.value })
              }
              autoComplete="off"
            />
            <TextField
              label="SMTP password"
              type="password"
              value={smtpPassword}
              onChange={(event) => setSmtpPassword(event.target.value)}
              autoComplete="new-password"
              placeholder={authDraft.smtp_password_configured ? 'Leave blank to keep current password' : ''}
              helperText={
                authDraft.smtp_password_configured
                  ? 'A password is configured. Leave this blank unless you want to replace it.'
                  : 'No SMTP password is configured.'
              }
            />
            <Stack direction={{ xs: 'column', md: 'row' }} spacing={2}>
              <TextField
                label="From email"
                type="email"
                value={authDraft.smtp_from_email}
                onChange={(event) =>
                  setAuthDraft({ ...authDraft, smtp_from_email: event.target.value })
                }
                sx={{ flex: 1 }}
              />
              <TextField
                label="From name"
                value={authDraft.smtp_from_name}
                onChange={(event) =>
                  setAuthDraft({ ...authDraft, smtp_from_name: event.target.value })
                }
                sx={{ flex: 1 }}
              />
            </Stack>

            {authDraft.smtp_security === 'none' && (
              <Alert severity="warning">
                Plain SMTP sends credentials and messages without transport encryption. Use it only for a trusted local relay.
              </Alert>
            )}

            <Stack direction="row" spacing={1.25} justifyContent="flex-end">
              <Button
                variant="outlined"
                disabled={
                  authBusy ||
                  !authDraft.smtp_host.trim() ||
                  !authDraft.smtp_from_email.trim() ||
                  (!authDraft.smtp_password_configured && !smtpPassword && !!authDraft.smtp_username)
                }
                onClick={async () => {
                  if (await saveAuthSettings()) await testSmtp()
                }}
              >
                Send test email
              </Button>
              <Button
                variant="contained"
                disabled={
                  authBusy ||
                  authDraft.smtp_port < 1 ||
                  authDraft.smtp_port > 65535 ||
                  (authDraft.registration_require_email_verification &&
                    (!authDraft.smtp_host.trim() || !authDraft.smtp_from_email.trim()))
                }
                onClick={saveAuthSettings}
              >
                Save authentication settings
              </Button>
            </Stack>
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
