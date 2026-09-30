import {
  Alert,
  Box,
  Button,
  Card,
  CardContent,
  CircularProgress,
  Link,
  Stack,
  TextField,
  Typography,
} from '@mui/material'
import { FormEvent, useEffect, useState } from 'react'
import { api } from '../api'
import { BrandMark, useBrand } from '../components/Branding'
import { ThemeModeButton } from '../colorMode'
import type { RegistrationConfig } from '../types'

type Props = {
  onAuthenticated: () => Promise<void>
}

type Mode = 'login' | 'register' | 'verify' | 'forgot' | 'reset'

export function LoginPage({ onAuthenticated }: Props) {
  const { brand } = useBrand()
  const [mode, setMode] = useState<Mode>('login')
  const [registration, setRegistration] = useState<RegistrationConfig>({
    enabled: false,
    require_email_verification: false,
  })
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [registerForm, setRegisterForm] = useState({
    username: '',
    email: '',
    displayName: '',
    password: '',
    confirm: '',
  })
  const [verificationEmail, setVerificationEmail] = useState('')
  const [verificationCode, setVerificationCode] = useState('')
  const [recoveryEmail, setRecoveryEmail] = useState('')
  const [recoveryCode, setRecoveryCode] = useState('')
  const [recoveryPassword, setRecoveryPassword] = useState('')
  const [recoveryConfirm, setRecoveryConfirm] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [notice, setNotice] = useState('')

  useEffect(() => {
    api.registrationConfig().then(setRegistration).catch(() => undefined)
  }, [])

  async function submitLogin(event: FormEvent) {
    event.preventDefault()
    setBusy(true)
    setError('')
    try {
      await api.login(username, password)
      const next = new URLSearchParams(window.location.search).get('continue')
      if (next && next.startsWith('/') && !next.startsWith('//')) {
        window.location.assign(next)
        return
      }
      await onAuthenticated()
    } catch {
      setError('Incorrect username, email, or password.')
    } finally {
      setBusy(false)
    }
  }

  async function submitRegistration(event: FormEvent) {
    event.preventDefault()
    setError('')
    setNotice('')
    if (registerForm.password !== registerForm.confirm) {
      setError('The passwords do not match.')
      return
    }
    setBusy(true)
    try {
      const result = await api.register({
        username: registerForm.username,
        email: registerForm.email,
        display_name: registerForm.displayName,
        password: registerForm.password,
      })
      if (result.verification_required) {
        setVerificationEmail(registerForm.email.trim())
        setVerificationCode('')
        setMode('verify')
        setNotice('A verification code was sent to your email address.')
      } else {
        setUsername(registerForm.username)
        setPassword('')
        setMode('login')
        setNotice('Account created. You can sign in now.')
      }
    } catch (err) {
      const status = (err as { status?: number }).status
      setError(
        status === 409
          ? 'That username or email is already in use.'
          : status === 503
            ? 'Could not send the verification email. Try again shortly.'
            : 'Could not create the account. Check the fields and use a password of at least 12 characters.',
      )
    } finally {
      setBusy(false)
    }
  }

  async function submitVerification(event: FormEvent) {
    event.preventDefault()
    setBusy(true)
    setError('')
    try {
      await api.verifyRegistration(verificationEmail, verificationCode)
      setUsername(verificationEmail)
      setPassword('')
      setMode('login')
      setNotice('Email verified. You can sign in now.')
    } catch {
      setError('The verification code is invalid or has expired.')
    } finally {
      setBusy(false)
    }
  }

  async function submitForgotPassword(event: FormEvent) {
    event.preventDefault()
    setBusy(true)
    setError('')
    setNotice('')
    try {
      await api.requestPasswordReset(recoveryEmail)
      setRecoveryCode('')
      setRecoveryPassword('')
      setRecoveryConfirm('')
      setMode('reset')
      setNotice('If an active account uses that email address, a six-digit reset code was sent.')
    } catch {
      setError('Could not request a password reset. Try again shortly.')
    } finally {
      setBusy(false)
    }
  }

  async function submitPasswordReset(event: FormEvent) {
    event.preventDefault()
    setError('')
    setNotice('')
    if (recoveryPassword !== recoveryConfirm) {
      setError('The passwords do not match.')
      return
    }
    setBusy(true)
    try {
      await api.resetPassword(recoveryEmail, recoveryCode, recoveryPassword)
      setUsername(recoveryEmail)
      setPassword('')
      setRecoveryCode('')
      setRecoveryPassword('')
      setRecoveryConfirm('')
      setMode('login')
      setNotice('Password reset. You can sign in now.')
    } catch {
      setError('The reset code is invalid or expired, or the new password is too short.')
    } finally {
      setBusy(false)
    }
  }

  const title =
    mode === 'login'
      ? 'Sign in to ' + brand.site_name
      : mode === 'register'
        ? 'Create your ' + brand.site_name + ' account'
        : mode === 'verify'
          ? 'Verify your email'
          : mode === 'forgot'
            ? 'Recover your account'
            : 'Set a new password'

  return (
    <Box
      sx={{
        minHeight: '100vh',
        display: 'grid',
        placeItems: 'center',
        px: 2,
        background: (theme) =>
          theme.palette.mode === 'dark'
            ? 'radial-gradient(circle at 20% 10%, rgba(66,133,244,.16), transparent 34%), #111318'
            : 'radial-gradient(circle at 20% 10%, rgba(211,227,253,.75), transparent 32%), #f8fafd',
      }}
    >
      <Box sx={{ position: 'fixed', top: 16, right: 16, zIndex: 2 }}>
        <ThemeModeButton />
      </Box>
      <Card sx={{ width: '100%', maxWidth: 500, borderRadius: 2.5 }}>
        <CardContent sx={{ p: { xs: 3.5, sm: 5 } }}>
          <Stack spacing={3}>
            <Box>
              <Box sx={{ mb: 3 }}>
                <BrandMark size={52} />
              </Box>
              <Typography variant="h4">{title}</Typography>
            </Box>

            {notice && <Alert severity="success">{notice}</Alert>}
            {error && <Alert severity="error">{error}</Alert>}

            {mode === 'login' && (
              <Box component="form" onSubmit={submitLogin}>
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
                  <Stack direction="row" alignItems="center" justifyContent="space-between">
                    <Stack direction="row" spacing={1.5}>
                      {registration.enabled && (
                        <Link
                          component="button"
                          type="button"
                          underline="hover"
                          onClick={() => {
                            setError('')
                            setNotice('')
                            setMode('register')
                          }}
                        >
                          Create account
                        </Link>
                      )}
                      {registration.enabled && registration.require_email_verification && (
                        <Link
                          component="button"
                          type="button"
                          underline="hover"
                          onClick={() => {
                            setError('')
                            setNotice('')
                            setVerificationEmail('')
                            setVerificationCode('')
                            setMode('verify')
                          }}
                        >
                          Verify email
                        </Link>
                      )}
                      <Link
                        component="button"
                        type="button"
                        underline="hover"
                        onClick={() => {
                          setError('')
                          setNotice('')
                          setRecoveryEmail(username.includes('@') ? username : '')
                          setMode('forgot')
                        }}
                      >
                        Forgot password?
                      </Link>
                    </Stack>
                    <Button
                      type="submit"
                      variant="contained"
                      size="large"
                      disabled={busy || !username || !password}
                      sx={{ minWidth: 112 }}
                    >
                      {busy ? <CircularProgress size={20} color="inherit" /> : 'Sign in'}
                    </Button>
                  </Stack>
                </Stack>
              </Box>
            )}

            {mode === 'register' && (
              <Box component="form" onSubmit={submitRegistration}>
                <Stack spacing={2}>
                  <TextField
                    label="Username"
                    value={registerForm.username}
                    onChange={(e) => setRegisterForm({ ...registerForm, username: e.target.value })}
                    autoComplete="username"
                    autoFocus
                  />
                  <TextField
                    label="Display name"
                    value={registerForm.displayName}
                    onChange={(e) =>
                      setRegisterForm({ ...registerForm, displayName: e.target.value })
                    }
                    autoComplete="name"
                  />
                  <TextField
                    label="Email"
                    type="email"
                    value={registerForm.email}
                    onChange={(e) => setRegisterForm({ ...registerForm, email: e.target.value })}
                    autoComplete="email"
                  />
                  <TextField
                    label="Password"
                    type="password"
                    value={registerForm.password}
                    onChange={(e) => setRegisterForm({ ...registerForm, password: e.target.value })}
                    autoComplete="new-password"
                    helperText="Use at least 12 characters."
                  />
                  <TextField
                    label="Confirm password"
                    type="password"
                    value={registerForm.confirm}
                    onChange={(e) => setRegisterForm({ ...registerForm, confirm: e.target.value })}
                    autoComplete="new-password"
                  />
                  {registration.require_email_verification && (
                    <Typography variant="body2" color="text.secondary">
                      You must verify your email address before the account can sign in.
                    </Typography>
                  )}
                  <Stack direction="row" alignItems="center" justifyContent="space-between">
                    <Link
                      component="button"
                      type="button"
                      underline="hover"
                      onClick={() => {
                        setError('')
                        setMode('login')
                      }}
                    >
                      Back to sign in
                    </Link>
                    <Button
                      type="submit"
                      variant="contained"
                      disabled={
                        busy ||
                        !registerForm.username ||
                        !registerForm.email ||
                        !registerForm.displayName ||
                        !registerForm.password ||
                        !registerForm.confirm
                      }
                    >
                      {busy ? <CircularProgress size={20} color="inherit" /> : 'Create account'}
                    </Button>
                  </Stack>
                </Stack>
              </Box>
            )}

            {mode === 'verify' && (
              <Box component="form" onSubmit={submitVerification}>
                <Stack spacing={2}>
                  <Typography color="text.secondary">
                    Enter the email address and the six-digit verification code.
                  </Typography>
                  <TextField
                    label="Email"
                    type="email"
                    value={verificationEmail}
                    onChange={(e) => setVerificationEmail(e.target.value)}
                    autoComplete="email"
                  />
                  <TextField
                    label="Verification code"
                    value={verificationCode}
                    onChange={(e) =>
                      setVerificationCode(e.target.value.replace(/\D/g, '').slice(0, 6))
                    }
                    inputProps={{ inputMode: 'numeric', maxLength: 6 }}
                    autoFocus
                  />
                  <Stack direction="row" alignItems="center" justifyContent="space-between">
                    <Button
                      variant="text"
                      disabled={busy || !verificationEmail}
                      onClick={async () => {
                        setError('')
                        try {
                          await api.resendRegistrationCode(verificationEmail)
                          setNotice('A new verification code was sent.')
                        } catch {
                          setError('Could not resend the verification code.')
                        }
                      }}
                    >
                      Resend code
                    </Button>
                    <Button
                      type="submit"
                      variant="contained"
                      disabled={busy || !verificationEmail || verificationCode.length !== 6}
                    >
                      {busy ? <CircularProgress size={20} color="inherit" /> : 'Verify email'}
                    </Button>
                  </Stack>
                </Stack>
              </Box>
            )}

            {mode === 'forgot' && (
              <Box component="form" onSubmit={submitForgotPassword}>
                <Stack spacing={2}>
                  <Typography color="text.secondary">
                    Enter the email address on your account. If it matches an active account, we will
                    send a six-digit reset code.
                  </Typography>
                  <TextField
                    label="Email"
                    type="email"
                    value={recoveryEmail}
                    onChange={(e) => setRecoveryEmail(e.target.value)}
                    autoComplete="email"
                    autoFocus
                  />
                  <Stack direction="row" alignItems="center" justifyContent="space-between">
                    <Link
                      component="button"
                      type="button"
                      underline="hover"
                      onClick={() => {
                        setError('')
                        setNotice('')
                        setMode('login')
                      }}
                    >
                      Back to sign in
                    </Link>
                    <Button type="submit" variant="contained" disabled={busy || !recoveryEmail}>
                      {busy ? <CircularProgress size={20} color="inherit" /> : 'Send reset code'}
                    </Button>
                  </Stack>
                </Stack>
              </Box>
            )}

            {mode === 'reset' && (
              <Box component="form" onSubmit={submitPasswordReset}>
                <Stack spacing={2}>
                  <TextField
                    label="Email"
                    type="email"
                    value={recoveryEmail}
                    onChange={(e) => setRecoveryEmail(e.target.value)}
                    autoComplete="email"
                  />
                  <TextField
                    label="Reset code"
                    value={recoveryCode}
                    onChange={(e) =>
                      setRecoveryCode(e.target.value.replace(/\D/g, '').slice(0, 6))
                    }
                    inputProps={{ inputMode: 'numeric', maxLength: 6 }}
                    autoFocus
                  />
                  <TextField
                    label="New password"
                    type="password"
                    value={recoveryPassword}
                    onChange={(e) => setRecoveryPassword(e.target.value)}
                    autoComplete="new-password"
                    helperText="Use at least 12 characters."
                  />
                  <TextField
                    label="Confirm new password"
                    type="password"
                    value={recoveryConfirm}
                    onChange={(e) => setRecoveryConfirm(e.target.value)}
                    autoComplete="new-password"
                  />
                  <Stack direction="row" alignItems="center" justifyContent="space-between">
                    <Button
                      variant="text"
                      disabled={busy || !recoveryEmail}
                      onClick={async () => {
                        setError('')
                        setNotice('')
                        try {
                          await api.requestPasswordReset(recoveryEmail)
                          setNotice(
                            'If an active account uses that email address, a new reset code was sent.',
                          )
                        } catch {
                          setError('Could not request a new reset code.')
                        }
                      }}
                    >
                      Resend code
                    </Button>
                    <Button
                      type="submit"
                      variant="contained"
                      disabled={
                        busy ||
                        !recoveryEmail ||
                        recoveryCode.length !== 6 ||
                        recoveryPassword.length < 12 ||
                        !recoveryConfirm
                      }
                    >
                      {busy ? <CircularProgress size={20} color="inherit" /> : 'Reset password'}
                    </Button>
                  </Stack>
                </Stack>
              </Box>
            )}
          </Stack>
        </CardContent>
      </Card>
    </Box>
  )
}
