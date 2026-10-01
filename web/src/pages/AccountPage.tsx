import {
  DevicesRounded,
  KeyRounded,
  LinkRounded,
  LogoutRounded,
  PersonRounded,
  SecurityRounded,
} from '@mui/icons-material'
import GitHubIcon from '@mui/icons-material/GitHub'
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
import type { ExternalIdentity, ExternalProvider, Me, Session } from '../types'

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
  const [externalProviders, setExternalProviders] = useState<ExternalProvider[]>([])
  const [externalIdentities, setExternalIdentities] = useState<ExternalIdentity[]>([])
  const [externalNotice, setExternalNotice] = useState('')
  const [externalError, setExternalError] = useState('')
  const [profileNotice, setProfileNotice] = useState('')
  const [passwordError, setPasswordError] = useState('')
  const [passwords, setPasswords] = useState({
    current: '',
    next: '',
    confirm: '',
  })
  const [emailChange, setEmailChange] = useState({ newEmail: '', currentPassword: '', code: '' })
  const [emailChangeRequested, setEmailChangeRequested] = useState(false)
  const [emailError, setEmailError] = useState('')
  const [emailNotice, setEmailNotice] = useState('')

  const loadSessions = useCallback(async () => {
    const result = await api.ownSessions()
    setSessions(result.items)
  }, [])

  const loadExternalAccounts = useCallback(async () => {
    const [providers, identities] = await Promise.all([
      api.externalProviders(),
      api.externalIdentities(),
    ])
    setExternalProviders(providers.items)
    setExternalIdentities(identities.items)
  }, [])

  useEffect(() => {
    setDisplayName(me.display_name)
    loadSessions()
    loadExternalAccounts().catch(() => undefined)

    const params = new URLSearchParams(window.location.search)
    if (params.get('external_result') === 'bound') {
      setExternalNotice('GitHub account linked.')
    }
    const errorCode = params.get('external_error')
    if (errorCode) {
      const messages: Record<string, string> = {
        identity_in_use: 'That GitHub account is already linked to another Auctor account.',
        provider_already_bound: 'A different GitHub account is already linked here. Unlink it first.',
        reauth_required: 'Your Auctor session changed while linking GitHub. Sign in again and retry.',
        invalid_flow: 'The GitHub linking request is invalid. Start the link again.',
        provider_denied: 'GitHub linking was cancelled.',
        provider_unavailable: 'GitHub linking is temporarily unavailable.',
        provider_failed: 'GitHub linking could not be completed. Try again.',
      }
      setExternalError(messages[errorCode] ?? 'GitHub linking could not be completed.')
    }
    if (params.has('external_result') || params.has('external_error') || params.has('provider')) {
      params.delete('external_result')
      params.delete('provider')
      params.delete('external_error')
      const next = params.toString()
      window.history.replaceState(null, '', window.location.pathname + (next ? '?' + next : ''))
    }
  }, [me.display_name, loadSessions, loadExternalAccounts])

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

  async function requestEmailChange() {
    setEmailError('')
    setEmailNotice('')
    try {
      await api.requestEmailChange(emailChange.newEmail, emailChange.currentPassword)
      setEmailChangeRequested(true)
      setEmailChange({ ...emailChange, currentPassword: '', code: '' })
      setEmailNotice('A verification code was sent to the new email address.')
    } catch (error) {
      const status = (error as { status?: number }).status
      setEmailError(
        status === 401
          ? 'The current password is incorrect.'
          : status === 409
            ? 'That email address is already in use.'
            : status === 503
              ? 'Could not send the verification email.'
              : 'Could not start the email change.',
      )
    }
  }

  async function confirmEmailChange() {
    setEmailError('')
    try {
      await api.confirmEmailChange(emailChange.newEmail, emailChange.code)
      setEmailChange({ newEmail: '', currentPassword: '', code: '' })
      setEmailChangeRequested(false)
      setEmailNotice('Email address updated.')
      await onRefresh()
    } catch {
      setEmailError('The verification code is invalid or has expired.')
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

      <Card sx={{ borderRadius: 2 }}>
        <CardContent sx={{ p: 3 }}>
          <Stack direction="row" spacing={1.5} alignItems="center" mb={2.5}>
            <Box
              sx={{
                width: 42,
                height: 42,
                borderRadius: 1.5,
                bgcolor: 'action.hover',
                color: 'secondary.main',
                display: 'grid',
                placeItems: 'center',
              }}
            >
              <LinkRounded />
            </Box>
            <Box>
              <Typography variant="h6">Connected accounts</Typography>
              <Typography variant="body2" color="text.secondary">
                Linked providers can sign in to this existing account. They can never create an account here.
              </Typography>
            </Box>
          </Stack>

          {externalNotice && (
            <Alert severity="success" sx={{ mb: 2 }} onClose={() => setExternalNotice('')}>
              {externalNotice}
            </Alert>
          )}
          {externalError && (
            <Alert severity="error" sx={{ mb: 2 }} onClose={() => setExternalError('')}>
              {externalError}
            </Alert>
          )}

          <Stack divider={<Divider flexItem />}>
            {Array.from(
              new Map(
                [
                  ...externalProviders,
                  ...externalIdentities.map((identity) => ({
                    id: identity.provider,
                    name: identity.provider === 'github' ? 'GitHub' : identity.provider,
                  })),
                ].map((provider) => [provider.id, provider]),
              ).values(),
            ).map((provider) => {
              const identity = externalIdentities.find((item) => item.provider === provider.id)
              return (
                <Stack
                  key={provider.id}
                  direction={{ xs: 'column', sm: 'row' }}
                  spacing={2}
                  alignItems={{ sm: 'center' }}
                  sx={{ py: 1.5 }}
                >
                  <Box
                    sx={{
                      width: 40,
                      height: 40,
                      borderRadius: 2.5,
                      bgcolor: 'action.hover',
                      display: 'grid',
                      placeItems: 'center',
                      flex: '0 0 auto',
                    }}
                  >
                    {provider.id === 'github' ? <GitHubIcon /> : <LinkRounded fontSize="small" />}
                  </Box>
                  <Box sx={{ flex: 1, minWidth: 0 }}>
                    <Stack direction="row" spacing={1} alignItems="center">
                      <Typography fontWeight={600}>{provider.name}</Typography>
                      <Chip
                        label={identity ? 'Linked' : 'Not linked'}
                        size="small"
                        color={identity ? 'success' : 'default'}
                        variant="outlined"
                      />
                    </Stack>
                    <Typography variant="body2" color="text.secondary" noWrap>
                      {identity
                        ? identity.login
                          ? '@' + identity.login
                          : 'Provider account linked'
                        : 'Link an existing provider account for faster sign-in.'}
                    </Typography>
                  </Box>
                  {identity ? (
                    <Button
                      color="error"
                      size="small"
                      onClick={async () => {
                        setExternalError('')
                        try {
                          await api.unlinkExternalIdentity(provider.id)
                          setExternalNotice(provider.name + ' account unlinked.')
                          await loadExternalAccounts()
                        } catch {
                          setExternalError('Could not unlink ' + provider.name + '.')
                        }
                      }}
                    >
                      Unlink
                    </Button>
                  ) : (
                    provider.id === 'github' &&
                    externalProviders.some((item) => item.id === 'github') && (
                      <Button
                        variant="outlined"
                        size="small"
                        startIcon={<GitHubIcon />}
                        onClick={() =>
                          window.location.assign(
                            api.externalAuthStartUrl('github', 'bind', '/account'),
                          )
                        }
                      >
                        Link GitHub
                      </Button>
                    )
                  )}
                </Stack>
              )
            })}
            {!externalProviders.length && !externalIdentities.length && (
              <Typography color="text.secondary" sx={{ py: 1 }}>
                No external sign-in providers are configured.
              </Typography>
            )}
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
              <TextField label="Current email" value={me.email} disabled />
              <Button
                variant="contained"
                sx={{ alignSelf: 'flex-end' }}
                disabled={!displayName.trim() || displayName === me.display_name}
                onClick={saveProfile}
              >
                Save profile
              </Button>

              <Divider sx={{ my: 0.5 }} />
              <Typography variant="subtitle2">Change email</Typography>
              {emailNotice && (
                <Alert severity="success" onClose={() => setEmailNotice('')}>
                  {emailNotice}
                </Alert>
              )}
              {emailError && <Alert severity="error">{emailError}</Alert>}
              <TextField
                label="New email"
                type="email"
                value={emailChange.newEmail}
                disabled={emailChangeRequested}
                onChange={(event) =>
                  setEmailChange({ ...emailChange, newEmail: event.target.value })
                }
                autoComplete="email"
              />
              {!emailChangeRequested ? (
                <>
                  <TextField
                    label="Current password"
                    type="password"
                    value={emailChange.currentPassword}
                    onChange={(event) =>
                      setEmailChange({ ...emailChange, currentPassword: event.target.value })
                    }
                    autoComplete="current-password"
                    helperText="We verify your password before sending a code to the new address."
                  />
                  <Button
                    variant="outlined"
                    sx={{ alignSelf: 'flex-end' }}
                    disabled={!emailChange.newEmail || !emailChange.currentPassword}
                    onClick={requestEmailChange}
                  >
                    Send verification code
                  </Button>
                </>
              ) : (
                <>
                  <TextField
                    label="Verification code"
                    value={emailChange.code}
                    onChange={(event) =>
                      setEmailChange({
                        ...emailChange,
                        code: event.target.value.replace(/\D/g, '').slice(0, 6),
                      })
                    }
                    inputProps={{ inputMode: 'numeric', maxLength: 6 }}
                  />
                  <Stack direction="row" spacing={1} justifyContent="flex-end">
                    <Button
                      variant="text"
                      onClick={() => {
                        setEmailChangeRequested(false)
                        setEmailChange({ ...emailChange, currentPassword: '', code: '' })
                        setEmailError('')
                      }}
                    >
                      Cancel
                    </Button>
                    <Button
                      variant="contained"
                      disabled={emailChange.code.length !== 6}
                      onClick={confirmEmailChange}
                    >
                      Confirm new email
                    </Button>
                  </Stack>
                </>
              )}
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
