import {
  AddRounded,
  BlockRounded,
  CloseRounded,
  KeyRounded,
  MoreVertRounded,
  RefreshRounded,
  SearchRounded,
} from '@mui/icons-material'
import {
  Alert,
  Box,
  Button,
  Chip,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  Divider,
  Drawer,
  FormControl,
  IconButton,
  InputAdornment,
  InputLabel,
  MenuItem,
  Select,
  Snackbar,
  Stack,
  Switch,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  TextField,
  Typography,
} from '@mui/material'
import { FormEvent, useEffect, useMemo, useState } from 'react'
import { api } from '../api'
import { UserAvatar } from '../components/UserAvatar'
import type { Role, User } from '../types'

function formatDate(value: string | null) {
  if (!value) return 'Never'
  return new Intl.DateTimeFormat(undefined, { dateStyle: 'medium', timeStyle: 'short' }).format(
    new Date(value),
  )
}

export function UsersPage() {
  const [users, setUsers] = useState<User[]>([])
  const [total, setTotal] = useState(0)
  const [query, setQuery] = useState('')
  const [status, setStatus] = useState('')
  const [selected, setSelected] = useState<User | null>(null)
  const [roles, setRoles] = useState<Role[]>([])
  const [addOpen, setAddOpen] = useState(false)
  const [notice, setNotice] = useState('')
  const [loading, setLoading] = useState(true)

  async function load() {
    setLoading(true)
    try {
      const result = await api.users(query, status)
      setUsers(result.items)
      setTotal(result.total)
      if (selected) {
        const updated = result.items.find((u) => u.id === selected.id)
        if (updated) setSelected(updated)
      }
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    const timer = setTimeout(load, 220)
    return () => clearTimeout(timer)
  }, [query, status])

  useEffect(() => {
    api.roles().then((r) => setRoles(r.items))
  }, [])

  return (
    <Stack spacing={2.5}>
      <Stack direction={{ xs: 'column', sm: 'row' }} justifyContent="space-between" gap={2}>
        <Box>
          <Typography variant="h4">Users</Typography>
          <Typography color="text.secondary" sx={{ mt: 0.5 }}>
            {total.toLocaleString()} accounts in the global user directory
          </Typography>
        </Box>
        <Button variant="contained" startIcon={<AddRounded />} onClick={() => setAddOpen(true)}>
          Add user
        </Button>
      </Stack>

      <Stack direction={{ xs: 'column', md: 'row' }} spacing={1.5}>
        <TextField
          placeholder="Search users"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          sx={{ flex: 1, maxWidth: 620 }}
          slotProps={{
            input: {
              startAdornment: (
                <InputAdornment position="start">
                  <SearchRounded color="action" />
                </InputAdornment>
              ),
            },
          }}
        />
        <FormControl sx={{ minWidth: 170 }}>
          <InputLabel>Status</InputLabel>
          <Select value={status} label="Status" onChange={(e) => setStatus(e.target.value)}>
            <MenuItem value="">All statuses</MenuItem>
            <MenuItem value="active">Active</MenuItem>
            <MenuItem value="disabled">Disabled</MenuItem>
          </Select>
        </FormControl>
        <IconButton onClick={load} aria-label="Refresh users">
          <RefreshRounded />
        </IconButton>
      </Stack>

      <TableContainer
        sx={{
          bgcolor: 'background.paper',
          border: '1px solid',
          borderColor: 'divider',
          borderRadius: 5,
          overflow: 'hidden',
          opacity: loading ? 0.65 : 1,
          transition: 'opacity .15s',
        }}
      >
        <Table>
          <TableHead>
            <TableRow sx={{ bgcolor: '#f8fafd' }}>
              <TableCell>User</TableCell>
              <TableCell>Roles</TableCell>
              <TableCell>Status</TableCell>
              <TableCell>Sessions</TableCell>
              <TableCell>Last activity</TableCell>
              <TableCell width={48} />
            </TableRow>
          </TableHead>
          <TableBody>
            {users.map((user) => (
              <TableRow
                key={user.id}
                hover
                onClick={() => setSelected(user)}
                sx={{ cursor: 'pointer' }}
              >
                <TableCell>
                  <Stack direction="row" spacing={1.5} alignItems="center">
                    <UserAvatar userId={user.id} name={user.display_name} />
                    <Box>
                      <Typography fontWeight={600}>{user.display_name}</Typography>
                      <Typography variant="body2" color="text.secondary">
                        {user.email} · @{user.username}
                      </Typography>
                    </Box>
                  </Stack>
                </TableCell>
                <TableCell>
                  <Stack direction="row" gap={0.75} flexWrap="wrap">
                    {user.roles.map((role) => (
                      <Chip key={role} label={role} size="small" variant="outlined" />
                    ))}
                  </Stack>
                </TableCell>
                <TableCell>
                  <Chip
                    size="small"
                    label={user.status === 'active' ? 'Active' : 'Disabled'}
                    color={user.status === 'active' ? 'success' : 'default'}
                    variant={user.status === 'active' ? 'filled' : 'outlined'}
                  />
                </TableCell>
                <TableCell>{user.active_sessions}</TableCell>
                <TableCell>
                  <Typography variant="body2" color="text.secondary">
                    {formatDate(user.last_seen_at)}
                  </Typography>
                </TableCell>
                <TableCell>
                  <IconButton size="small">
                    <MoreVertRounded fontSize="small" />
                  </IconButton>
                </TableCell>
              </TableRow>
            ))}
            {!users.length && !loading && (
              <TableRow>
                <TableCell colSpan={6} align="center" sx={{ py: 7, color: 'text.secondary' }}>
                  No users match these filters.
                </TableCell>
              </TableRow>
            )}
          </TableBody>
        </Table>
      </TableContainer>

      <CreateUserDialog
        open={addOpen}
        roles={roles}
        onClose={() => setAddOpen(false)}
        onCreated={async () => {
          setAddOpen(false)
          setNotice('User created')
          await load()
        }}
      />

      <UserDrawer
        user={selected}
        roles={roles}
        onClose={() => setSelected(null)}
        onChanged={async (message) => {
          setNotice(message)
          await load()
        }}
      />

      <Snackbar
        open={Boolean(notice)}
        autoHideDuration={3000}
        onClose={() => setNotice('')}
        message={notice}
      />
    </Stack>
  )
}

function CreateUserDialog({
  open,
  roles,
  onClose,
  onCreated,
}: {
  open: boolean
  roles: Role[]
  onClose: () => void
  onCreated: () => Promise<void>
}) {
  const [form, setForm] = useState({
    username: '',
    display_name: '',
    email: '',
    password: '',
    roles: ['user'],
  })
  const [error, setError] = useState(false)
  const [busy, setBusy] = useState(false)

  async function submit(event: FormEvent) {
    event.preventDefault()
    setBusy(true)
    setError(false)
    try {
      await api.createUser(form)
      setForm({ username: '', display_name: '', email: '', password: '', roles: ['user'] })
      await onCreated()
    } catch {
      setError(true)
    } finally {
      setBusy(false)
    }
  }

  return (
    <Dialog open={open} onClose={onClose} fullWidth maxWidth="sm">
      <Box component="form" onSubmit={submit}>
        <DialogTitle>Create user</DialogTitle>
        <DialogContent>
          <Stack spacing={2} sx={{ pt: 1 }}>
            {error && <Alert severity="error">Could not create this account.</Alert>}
            <TextField
              label="Display name"
              value={form.display_name}
              onChange={(e) => setForm({ ...form, display_name: e.target.value })}
              required
            />
            <TextField
              label="Username"
              value={form.username}
              onChange={(e) => setForm({ ...form, username: e.target.value })}
              required
            />
            <TextField
              label="Email"
              type="email"
              value={form.email}
              onChange={(e) => setForm({ ...form, email: e.target.value })}
              required
            />
            <TextField
              label="Temporary password"
              type="password"
              helperText="At least 12 characters"
              value={form.password}
              onChange={(e) => setForm({ ...form, password: e.target.value })}
              required
            />
            <FormControl>
              <InputLabel>Roles</InputLabel>
              <Select
                multiple
                label="Roles"
                value={form.roles}
                onChange={(e) =>
                  setForm({
                    ...form,
                    roles:
                      typeof e.target.value === 'string'
                        ? e.target.value.split(',')
                        : e.target.value,
                  })
                }
              >
                {roles.map((role) => (
                  <MenuItem key={role.id} value={role.name}>
                    {role.name}
                  </MenuItem>
                ))}
              </Select>
            </FormControl>
          </Stack>
        </DialogContent>
        <DialogActions sx={{ p: 3, pt: 1 }}>
          <Button onClick={onClose}>Cancel</Button>
          <Button type="submit" variant="contained" disabled={busy}>
            Create account
          </Button>
        </DialogActions>
      </Box>
    </Dialog>
  )
}

function UserDrawer({
  user,
  roles,
  onClose,
  onChanged,
}: {
  user: User | null
  roles: Role[]
  onClose: () => void
  onChanged: (message: string) => Promise<void>
}) {
  const [draft, setDraft] = useState<User | null>(user)
  const [password, setPassword] = useState('')

  useEffect(() => setDraft(user), [user])

  const hasChanges = useMemo(
    () =>
      user &&
      draft &&
      (user.display_name !== draft.display_name ||
        user.username !== draft.username ||
        user.email !== draft.email ||
        user.status !== draft.status ||
        JSON.stringify([...user.roles].sort()) !== JSON.stringify([...draft.roles].sort())),
    [user, draft],
  )

  if (!draft) return null

  async function save() {
    await api.updateUser(draft!.id, {
      display_name: draft!.display_name,
      username: draft!.username,
      email: draft!.email,
      status: draft!.status,
    })
    await api.setRoles(draft!.id, draft!.roles)
    await onChanged('User updated')
  }

  return (
    <Drawer
      anchor="right"
      open={Boolean(user)}
      onClose={onClose}
      slotProps={{ paper: { sx: { width: { xs: '100%', sm: 520 }, p: 0 } } }}
    >
      <Stack sx={{ height: '100%' }}>
        <Stack direction="row" alignItems="center" justifyContent="space-between" sx={{ p: 2.5 }}>
          <Typography variant="h6">User details</Typography>
          <IconButton onClick={onClose}>
            <CloseRounded />
          </IconButton>
        </Stack>
        <Divider />
        <Stack spacing={3} sx={{ p: 3, overflowY: 'auto', flex: 1 }}>
          <Stack direction="row" spacing={2} alignItems="center">
            <UserAvatar userId={draft.id} name={draft.display_name} size={64} />
            <Box>
              <Typography variant="h6">{draft.display_name}</Typography>
              <Typography color="text.secondary">@{draft.username}</Typography>
            </Box>
          </Stack>

          <Stack spacing={2}>
            <TextField
              label="Display name"
              value={draft.display_name}
              onChange={(e) => setDraft({ ...draft, display_name: e.target.value })}
            />
            <TextField
              label="Username"
              value={draft.username}
              onChange={(e) => setDraft({ ...draft, username: e.target.value })}
            />
            <TextField
              label="Email"
              value={draft.email}
              onChange={(e) => setDraft({ ...draft, email: e.target.value })}
            />
          </Stack>

          <Box>
            <Typography fontWeight={600} sx={{ mb: 1.2 }}>
              Account status
            </Typography>
            <Stack direction="row" alignItems="center" justifyContent="space-between">
              <Box>
                <Typography>{draft.status === 'active' ? 'Active' : 'Disabled'}</Typography>
                <Typography variant="body2" color="text.secondary">
                  Disabled users cannot sign in.
                </Typography>
              </Box>
              <Switch
                checked={draft.status === 'active'}
                onChange={(e) =>
                  setDraft({ ...draft, status: e.target.checked ? 'active' : 'disabled' })
                }
              />
            </Stack>
          </Box>

          <Box>
            <Typography fontWeight={600} sx={{ mb: 1.2 }}>
              Roles
            </Typography>
            <Stack direction="row" gap={1} flexWrap="wrap">
              {roles.map((role) => {
                const selected = draft.roles.includes(role.name)
                return (
                  <Chip
                    key={role.id}
                    label={role.name}
                    color={selected ? 'primary' : 'default'}
                    variant={selected ? 'filled' : 'outlined'}
                    onClick={() =>
                      setDraft({
                        ...draft,
                        roles: selected
                          ? draft.roles.filter((r) => r !== role.name)
                          : [...draft.roles, role.name],
                      })
                    }
                  />
                )
              })}
            </Stack>
          </Box>

          <Divider />

          <Box>
            <Typography fontWeight={600}>Sessions</Typography>
            <Typography variant="body2" color="text.secondary" sx={{ mb: 1.5 }}>
              {draft.active_sessions} active · Last activity {formatDate(draft.last_seen_at)}
            </Typography>
            <Button
              variant="outlined"
              startIcon={<BlockRounded />}
              onClick={async () => {
                const result = await api.revokeSessions(draft.id)
                await onChanged('Revoked ' + result.revoked + ' sessions')
              }}
            >
              Revoke all sessions
            </Button>
          </Box>

          <Box>
            <Typography fontWeight={600}>Reset password</Typography>
            <Typography variant="body2" color="text.secondary" sx={{ mb: 1.5 }}>
              Resetting a password also revokes all sessions.
            </Typography>
            <Stack direction="row" spacing={1}>
              <TextField
                fullWidth
                size="small"
                type="password"
                label="New password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
              />
              <Button
                variant="outlined"
                startIcon={<KeyRounded />}
                disabled={password.length < 12}
                onClick={async () => {
                  await api.setPassword(draft.id, password)
                  setPassword('')
                  await onChanged('Password reset')
                }}
              >
                Reset
              </Button>
            </Stack>
          </Box>

          <Typography variant="caption" color="text.secondary">
            Created {formatDate(draft.created_at)}
          </Typography>
        </Stack>

        <Divider />
        <Stack direction="row" justifyContent="flex-end" spacing={1} sx={{ p: 2.5 }}>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="contained" disabled={!hasChanges} onClick={save}>
            Save changes
          </Button>
        </Stack>
      </Stack>
    </Drawer>
  )
}
