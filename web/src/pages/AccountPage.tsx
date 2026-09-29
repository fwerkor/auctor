import {
  DevicesRounded,
  KeyRounded,
  LogoutRounded,
  PersonRounded,
  SecurityRounded,
} from '@mui/icons-material'
import {
  Alert,
  Box,
  Button,
  Card,
  CardContent,
  Chip,
  Divider,
  Stack,
  TextField,
  Typography,
} from '@mui/material'
import { FormEvent, useCallback, useEffect, useState } from 'react'
import { api } from '../api'
import { BrandMark, useBrand } from '../components/Branding'
import { ThemeModeButton } from '../colorMode'
import { UserAvatar } from '../components/UserAvatar'
import type { Me, Session } from '../types'

function formatDate(value: string) {
  return new Intl.DateTimeFormat(undefined, { dateStyle: 'medium', timeStyle: 'short' }).format(
    new Date(value),
  )
}

function sessionLabel(userAgent: string | null) {
  if (!userAgent) return 'Unknown client'
  if (userAgent.includes('Edg/')) return 'Microsoft Edge'
  if (userAgent.includes('Chrome/')) return 'Google Chrome'
  if (userAgent.includes('Firefox/')) return 'Firefox'
  if (userAgent.includes('Safari/') && !userAgent.includes('Chrome/')) return 'Safari'
  if (userAgent.includes('curl/')) return 'curl'
  return 'Browser or API client'
}

export function AccountPage({
  me,
  standalone = false,
  onRefresh,
  onSignedOut,
}: {
  me: Me
  standalone?: boolean
  onRefresh: () => Promise<void>
  onSignedOut: () => void
}) {
  const { brand } = useBrand()
  const [displayName, setDisplayName] = useState(me.display_name)
  const [sessions, setSessions] = useState<Session[]>([])
  const [profileNotice, setProfileNotice] = useState('')
  const [passwordError, setPasswordError] = useState('')
  const [passwords, setPasswords] = useState({
    current: '',
    next: '',
    confirm: '',
  })

  const loadSessions = useCallback(async () => {
    const result = await api.ownSessions()
    setSessions(result.items)
  }, [])

  useEffect(() => {
    setDisplayName(me.display_name)
    loadSessions()
  }, [me.display_name, loadSessions])

  async function saveProfile() {
    await api.updateProfile(displayName)
    setProfileNotice('Profile updated')
    await onRefresh()
  }

  async function changePassword(event: FormEvent) {
    event.preventDefault()
    setPasswordError('')
    if (passwords.next !== passwords.confirm) {
      setPasswordError('The new passwords do not match.')
      return
    }
    try {
      await api.changePassword(passwords.current, passwords.next)
      setPasswords({ current: '', next: '', confirm: '' })
      onSignedOut()
    } catch (error) {
      const status = (error as { status?: number }).status
      setPasswordError(
        status === 401
          ? 'The current password is incorrect.'
          : 'Could not change the password. Use at least 12 characters and choose a new value.',
      )
    }
  }

  const content = (
    <Stack spacing={3} sx={{ width: '100%', maxWidth: 980, mx: 'auto' }}>
      <Box>
        <Typography variant="h4">My account</Typography>
        <Typography color="text.secondary" sx={{ mt: 0.5 }}>
          Manage your {brand.site_name} identity, access context, and signed-in devices.
        </Typography>
      </Box>

      <Card sx={{ borderRadius: 2 }}>
        <CardContent sx={{ p: { xs: 2.5, sm: 3.5 } }}>
          <Stack direction={{ xs: 'column', sm: 'row' }} spacing={2.5} alignItems={{ sm: 'center' }}>
            <UserAvatar userId={me.id} email={me.email} name={me.display_name} size={80} />
            <Box sx={{ flex: 1 }}>
              <Typography variant="h5">{me.display_name}</Typography>
              <Typography color="text.secondary" sx={{ mt: 0.3 }}>
                @{me.username} · {me.email}
              </Typography>
              <Stack direction="row" gap={0.75} flexWrap="wrap" sx={{ mt: 1.5 }}>
                {me.roles.map((role) => (
                  <Chip key={role} label={role} size="small" variant="outlined" />
                ))}
                {me.groups.map((group) => (
                  <Chip
                    key={group}
                    label={group}
                    size="small"
                    sx={{ bgcolor: 'action.hover', color: 'success.main' }}
                  />
                ))}
              </Stack>
            </Box>
          </Stack>
        </CardContent>
      </Card>

      <Stack direction={{ xs: 'column', lg: 'row' }} spacing={2.5} alignItems="stretch">
        <Card sx={{ borderRadius: 2, flex: 1 }}>
          <CardContent sx={{ p: 3 }}>
            <Stack direction="row" spacing={1.5} alignItems="center" mb={2.5}>
              <Box
                sx={{
                  width: 42,
                  height: 42,
                  borderRadius: 1.5,
                  bgcolor: 'action.hover',
                  color: 'primary.main',
                  display: 'grid',
                  placeItems: 'center',
                }}
              >
                <PersonRounded />
              </Box>
              <Box>
                <Typography variant="h6">Profile</Typography>
                <Typography variant="body2" color="text.secondary">
                  Your public name inside connected services.
                </Typography>
              </Box>
            </Stack>

            {profileNotice && (
              <Alert severity="success" sx={{ mb: 2 }} onClose={() => setProfileNotice('')}>
                {profileNotice}
              </Alert>
            )}

            <Stack spacing={2}>
              <TextField
                label="Display name"
                value={displayName}
                onChange={(event) => setDisplayName(event.target.value)}
              />
              <TextField label="Username" value={me.username} disabled />
              <TextField
                label="Email"
                value={me.email}
                disabled
                helperText="Email changes will be enabled together with verification."
              />
              <Button
                variant="contained"
                sx={{ alignSelf: 'flex-end' }}
                disabled={!displayName.trim() || displayName === me.display_name}
                onClick={saveProfile}
              >
                Save profile
              </Button>
            </Stack>
          </CardContent>
        </Card>

        <Card sx={{ borderRadius: 2, flex: 1 }}>
          <CardContent sx={{ p: 3 }}>
            <Stack direction="row" spacing={1.5} alignItems="center" mb={2.5}>
              <Box
                sx={{
                  width: 42,
                  height: 42,
                  borderRadius: 1.5,
                  bgcolor: 'action.hover',
                  color: 'error.main',
                  display: 'grid',
                  placeItems: 'center',
                }}
              >
                <KeyRounded />
              </Box>
              <Box>
                <Typography variant="h6">Password</Typography>
                <Typography variant="body2" color="text.secondary">
                  Changing it signs out all existing sessions.
                </Typography>
              </Box>
            </Stack>

            <Stack component="form" spacing={2} onSubmit={changePassword}>
              {passwordError && <Alert severity="error">{passwordError}</Alert>}
              <TextField
                label="Current password"
                type="password"
                autoComplete="current-password"
                value={passwords.current}
                onChange={(event) => setPasswords({ ...passwords, current: event.target.value })}
              />
              <TextField
                label="New password"
                type="password"
                autoComplete="new-password"
                value={passwords.next}
                onChange={(event) => setPasswords({ ...passwords, next: event.target.value })}
                helperText="At least 12 characters."
              />
              <TextField
                label="Confirm new password"
                type="password"
                autoComplete="new-password"
                value={passwords.confirm}
                onChange={(event) => setPasswords({ ...passwords, confirm: event.target.value })}
              />
              <Button
                type="submit"
                variant="outlined"
                sx={{ alignSelf: 'flex-end' }}
                disabled={
                  !passwords.current ||
                  passwords.next.length < 12 ||
                  !passwords.confirm
                }
              >
                Change password
              </Button>
            </Stack>
          </CardContent>
        </Card>
      </Stack>

      <Card sx={{ borderRadius: 2 }}>
        <CardContent sx={{ p: 3 }}>
          <Stack direction="row" spacing={1.5} alignItems="center" mb={2.5}>
            <Box
              sx={{
                width: 42,
                height: 42,
                borderRadius: 1.5,
                bgcolor: 'action.hover',
                color: 'success.main',
                display: 'grid',
                placeItems: 'center',
              }}
            >
              <DevicesRounded />
            </Box>
            <Box>
              <Typography variant="h6">Sessions and devices</Typography>
              <Typography variant="body2" color="text.secondary">
                Review recent sign-ins and remove devices you no longer trust.
              </Typography>
            </Box>
          </Stack>

          <Stack divider={<Divider flexItem />} spacing={0}>
            {sessions.map((session) => {
              const current = session.id === me.session_id
              return (
                <Stack
                  key={session.id}
                  direction={{ xs: 'column', sm: 'row' }}
                  spacing={2}
                  alignItems={{ sm: 'center' }}
                  sx={{ py: 2 }}
                >
                  <Box
                    sx={{
                      width: 40,
                      height: 40,
                      borderRadius: 2.5,
                      bgcolor: 'action.hover',
                      color: 'text.secondary',
                      display: 'grid',
                      placeItems: 'center',
                      flex: '0 0 auto',
                    }}
                  >
                    <SecurityRounded fontSize="small" />
                  </Box>
                  <Box sx={{ flex: 1, minWidth: 0 }}>
                    <Stack direction="row" gap={1} alignItems="center" flexWrap="wrap">
                      <Typography fontWeight={600}>{sessionLabel(session.user_agent)}</Typography>
                      {current && <Chip label="This device" size="small" color="primary" />}
                      <Chip
                        label={session.active ? 'Active' : 'Ended'}
                        size="small"
                        color={session.active ? 'success' : 'default'}
                        variant="outlined"
                      />
                    </Stack>
                    <Typography variant="body2" color="text.secondary">
                      Last seen {formatDate(session.last_seen_at)}
                      {session.ip ? ' · ' + session.ip : ''}
                    </Typography>
                    <Typography
                      variant="caption"
                      color="text.secondary"
                      display="block"
                      noWrap
                      title={session.user_agent ?? undefined}
                    >
                      {session.user_agent ?? 'No client details'}
                    </Typography>
                  </Box>
                  {session.active && (
                    <Button
                      color="error"
                      size="small"
                      startIcon={current ? <LogoutRounded /> : undefined}
                      onClick={async () => {
                        await api.revokeOwnSession(session.id)
                        if (current) {
                          onSignedOut()
                        } else {
                          await loadSessions()
                        }
                      }}
                    >
                      {current ? 'Sign out' : 'Revoke'}
                    </Button>
                  )}
                </Stack>
              )
            })}
            {!sessions.length && (
              <Typography color="text.secondary" sx={{ py: 3 }}>
                No sessions are recorded for this account.
              </Typography>
            )}
          </Stack>
        </CardContent>
      </Card>
    </Stack>
  )

  if (!standalone) return content

  return (
    <Box sx={{ minHeight: '100vh', bgcolor: 'background.default' }}>
      <Stack
        direction="row"
        alignItems="center"
        justifyContent="space-between"
        sx={{
          height: 72,
          px: { xs: 2, sm: 3 },
          borderBottom: '1px solid',
          borderColor: 'divider',
          bgcolor: (theme) =>
            theme.palette.mode === 'dark' ? 'rgba(17,19,24,.95)' : 'rgba(248,250,253,.95)',
        }}
      >
        <Stack direction="row" alignItems="center" spacing={1.25}>
          <BrandMark size={36} />
          <Typography fontWeight={600}>{brand.site_name}</Typography>
        </Stack>
        <Stack direction="row" alignItems="center" spacing={0.5}>
          <ThemeModeButton />
          <Button
            color="inherit"
            startIcon={<LogoutRounded />}
            onClick={async () => {
              await api.logout()
              onSignedOut()
            }}
          >
            Sign out
          </Button>
        </Stack>
      </Stack>
      <Box sx={{ p: { xs: 2, sm: 3, lg: 4 } }}>{content}</Box>
    </Box>
  )
}
