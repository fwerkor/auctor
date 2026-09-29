import {
  AddRounded,
  BlockRounded,
  CheckCircleOutlineRounded,
  CloseRounded,
  DevicesRounded,
  KeyRounded,
  PersonOutlineRounded,
  RefreshRounded,
  SearchRounded,
  SecurityRounded,
} from '@mui/icons-material'
import {
  Alert,
  Box,
  Button,
  Checkbox,
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
  Tab,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Tabs,
  TextField,
  Tooltip,
  Typography,
} from '@mui/material'
import { FormEvent, useCallback, useEffect, useMemo, useState } from 'react'
import { api } from '../api'
import { UserAvatar } from '../components/UserAvatar'
import type { Group, Role, Session, User } from '../types'

function formatDate(value: string | null) {
  if (!value) return 'Never'
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

export function UsersPage() {
  const [users, setUsers] = useState<User[]>([])
  const [total, setTotal] = useState(0)
  const [query, setQuery] = useState('')
  const [status, setStatus] = useState('')
  const [selected, setSelected] = useState<User | null>(null)
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set())
  const [roles, setRoles] = useState<Role[]>([])
  const [groups, setGroups] = useState<Group[]>([])
  const [addOpen, setAddOpen] = useState(false)
  const [notice, setNotice] = useState('')
  const [loading, setLoading] = useState(true)

  const load = useCallback(async () => {
    setLoading(true)
    try {
      const result = await api.users(query, status)
      setUsers(result.items)
      setTotal(result.total)
      setSelected((current) => {
        if (!current) return null
        return result.items.find((user) => user.id === current.id) ?? current
      })
    } finally {
      setLoading(false)
    }
  }, [query, status])

  useEffect(() => {
    const timer = setTimeout(load, 220)
    return () => clearTimeout(timer)
  }, [load])

  useEffect(() => {
    Promise.all([api.roles(), api.groups()]).then(([roleResult, groupResult]) => {
      setRoles(roleResult.items)
      setGroups(groupResult.items)
    })
  }, [])

  const visibleIds = users.map((user) => user.id)
  const allVisibleSelected =
    visibleIds.length > 0 && visibleIds.every((id) => selectedIds.has(id))
  const someVisibleSelected = visibleIds.some((id) => selectedIds.has(id)) && !allVisibleSelected

  async function bulk(action: 'activate' | 'disable' | 'revoke_sessions') {
    const ids = [...selectedIds]
    if (!ids.length) return
    const result = await api.bulkUsers(ids, action)
    setNotice(
      action === 'revoke_sessions'
        ? 'Revoked ' + result.affected + ' sessions'
        : 'Updated ' + result.affected + ' users',
    )
    setSelectedIds(new Set())
    await load()
  }

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
          placeholder="Search name, username, or email"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
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
          <Select value={status} label="Status" onChange={(event) => setStatus(event.target.value)}>
            <MenuItem value="">All statuses</MenuItem>
            <MenuItem value="active">Active</MenuItem>
            <MenuItem value="disabled">Disabled</MenuItem>
          </Select>
        </FormControl>
        <Tooltip title="Refresh">
          <IconButton onClick={load} aria-label="Refresh users">
            <RefreshRounded />
          </IconButton>
        </Tooltip>
      </Stack>

      {selectedIds.size > 0 && (
        <Stack
          direction={{ xs: 'column', sm: 'row' }}
          spacing={1}
          alignItems={{ sm: 'center' }}
          sx={{
            px: 2,
            py: 1.25,
            borderRadius: 1.5,
            bgcolor: '#e8f0fe',
            color: '#0842a0',
          }}
        >
          <Typography fontWeight={600} sx={{ mr: { sm: 'auto' } }}>
            {selectedIds.size} selected
          </Typography>
          <Button
            size="small"
            startIcon={<CheckCircleOutlineRounded />}
            onClick={() => bulk('activate')}
          >
            Activate
          </Button>
          <Button size="small" startIcon={<BlockRounded />} onClick={() => bulk('disable')}>
            Disable
          </Button>
          <Button size="small" startIcon={<DevicesRounded />} onClick={() => bulk('revoke_sessions')}>
            Revoke sessions
          </Button>
        </Stack>
      )}

      <TableContainer
        sx={{
          bgcolor: 'background.paper',
          border: '1px solid',
          borderColor: 'divider',
          borderRadius: 2,
          overflow: 'hidden',
          opacity: loading ? 0.65 : 1,
          transition: 'opacity .15s',
        }}
      >
        <Table>
          <TableHead>
            <TableRow sx={{ bgcolor: '#f8fafd' }}>
              <TableCell padding="checkbox">
                <Checkbox
                  checked={allVisibleSelected}
                  indeterminate={someVisibleSelected}
                  onChange={(event) => {
                    setSelectedIds((current) => {
                      const next = new Set(current)
                      for (const id of visibleIds) {
                        if (event.target.checked) next.add(id)
                        else next.delete(id)
                      }
                      return next
                    })
                  }}
                />
              </TableCell>
              <TableCell>User</TableCell>
              <TableCell>Access</TableCell>
              <TableCell>Status</TableCell>
              <TableCell>Sessions</TableCell>
              <TableCell>Last activity</TableCell>
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
                <TableCell padding="checkbox" onClick={(event) => event.stopPropagation()}>
                  <Checkbox
                    checked={selectedIds.has(user.id)}
                    onChange={(event) => {
                      setSelectedIds((current) => {
                        const next = new Set(current)
                        if (event.target.checked) next.add(user.id)
                        else next.delete(user.id)
                        return next
                      })
                    }}
                  />
                </TableCell>
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
                  <Stack direction="row" gap={0.6} flexWrap="wrap">
                    {user.roles.slice(0, 2).map((role) => (
                      <Chip key={'role-' + role} label={role} size="small" variant="outlined" />
                    ))}
                    {user.groups.slice(0, 2).map((group) => (
                      <Chip
                        key={'group-' + group}
                        label={group}
                        size="small"
                        sx={{ bgcolor: '#e6f4ea', color: '#137333' }}
                      />
                    ))}
                    {user.roles.length + user.groups.length > 4 && (
                      <Chip
                        label={'+' + (user.roles.length + user.groups.length - 4)}
                        size="small"
                        variant="outlined"
                      />
                    )}
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
        groups={groups}
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
        groups={groups}
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
  groups,
  onClose,
  onCreated,
}: {
  open: boolean
  roles: Role[]
  groups: Group[]
  onClose: () => void
  onCreated: () => Promise<void>
}) {
  const [form, setForm] = useState({
    username: '',
    display_name: '',
    email: '',
    password: '',
    roles: ['user'],
    groups: [] as string[],
  })
  const [error, setError] = useState(false)
  const [busy, setBusy] = useState(false)

  async function submit(event: FormEvent) {
    event.preventDefault()
    setBusy(true)
    setError(false)
    try {
      const result = await api.createUser({
        username: form.username,
        display_name: form.display_name,
        email: form.email,
        password: form.password,
        roles: form.roles,
      })
      if (form.groups.length) {
        await api.setGroups(result.id, form.groups)
      }
      setForm({
        username: '',
        display_name: '',
        email: '',
        password: '',
        roles: ['user'],
        groups: [],
      })
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
              onChange={(event) => setForm({ ...form, display_name: event.target.value })}
              required
            />
            <Stack direction={{ xs: 'column', sm: 'row' }} spacing={2}>
              <TextField
                fullWidth
                label="Username"
                value={form.username}
                onChange={(event) => setForm({ ...form, username: event.target.value })}
                required
              />
              <TextField
                fullWidth
                label="Email"
                type="email"
                value={form.email}
                onChange={(event) => setForm({ ...form, email: event.target.value })}
                required
              />
            </Stack>
            <TextField
              label="Temporary password"
              type="password"
              helperText="At least 12 characters"
              value={form.password}
              onChange={(event) => setForm({ ...form, password: event.target.value })}
              required
            />
            <FormControl>
              <InputLabel>Roles</InputLabel>
              <Select
                multiple
                label="Roles"
                value={form.roles}
                onChange={(event) =>
                  setForm({
                    ...form,
                    roles:
                      typeof event.target.value === 'string'
                        ? event.target.value.split(',')
                        : event.target.value,
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
            <FormControl>
              <InputLabel>Groups</InputLabel>
              <Select
                multiple
                label="Groups"
                value={form.groups}
                onChange={(event) =>
                  setForm({
                    ...form,
                    groups:
                      typeof event.target.value === 'string'
                        ? event.target.value.split(',')
                        : event.target.value,
                  })
                }
              >
                {groups.map((group) => (
                  <MenuItem key={group.id} value={group.id}>
                    {group.name}
                  </MenuItem>
                ))}
              </Select>
            </FormControl>
          </Stack>
        </DialogContent>
        <DialogActions sx={{ p: 3, pt: 1 }}>
          <Button onClick={onClose}>Cancel</Button>
          <Button type="submit" variant="contained" disabled={busy || form.password.length < 12}>
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
  groups,
  onClose,
  onChanged,
}: {
  user: User | null
  roles: Role[]
  groups: Group[]
  onClose: () => void
  onChanged: (message: string) => Promise<void>
}) {
  const [draft, setDraft] = useState<User | null>(user)
  const [password, setPassword] = useState('')
  const [tab, setTab] = useState(0)
  const [sessions, setSessions] = useState<Session[]>([])

  const loadSessions = useCallback(async (userId: string) => {
    const result = await api.sessions(userId)
    setSessions(result.items)
  }, [])

  useEffect(() => {
    setDraft(user)
    setTab(0)
    setPassword('')
    if (user) loadSessions(user.id)
    else setSessions([])
  }, [user, loadSessions])

  const hasChanges = useMemo(
    () =>
      user &&
      draft &&
      (user.display_name !== draft.display_name ||
        user.username !== draft.username ||
        user.email !== draft.email ||
        user.status !== draft.status ||
        JSON.stringify([...user.roles].sort()) !== JSON.stringify([...draft.roles].sort()) ||
        JSON.stringify([...user.groups].sort()) !== JSON.stringify([...draft.groups].sort())),
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
    await api.setGroups(
      draft!.id,
      groups.filter((group) => draft!.groups.includes(group.name)).map((group) => group.id),
    )
    await onChanged('User updated')
  }

  return (
    <Drawer
      anchor="right"
      open={Boolean(user)}
      onClose={onClose}
      slotProps={{ paper: { sx: { width: { xs: '100%', sm: 580 }, p: 0 } } }}
    >
      <Stack sx={{ height: '100%' }}>
        <Stack direction="row" alignItems="center" justifyContent="space-between" sx={{ p: 2.5 }}>
          <Stack direction="row" spacing={1.5} alignItems="center">
            <UserAvatar userId={draft.id} name={draft.display_name} size={44} />
            <Box>
              <Typography fontWeight={600}>{draft.display_name}</Typography>
              <Typography variant="body2" color="text.secondary">
                @{draft.username}
              </Typography>
            </Box>
          </Stack>
          <IconButton onClick={onClose}>
            <CloseRounded />
          </IconButton>
        </Stack>

        <Tabs
          value={tab}
          onChange={(_, value: number) => setTab(value)}
          sx={{ px: 2.5, borderBottom: '1px solid', borderColor: 'divider' }}
        >
          <Tab icon={<PersonOutlineRounded />} iconPosition="start" label="Profile" />
          <Tab icon={<SecurityRounded />} iconPosition="start" label="Access" />
          <Tab icon={<DevicesRounded />} iconPosition="start" label="Security" />
        </Tabs>

        <Box sx={{ overflowY: 'auto', flex: 1 }}>
          {tab === 0 && (
            <Stack spacing={3} sx={{ p: 3 }}>
              <Stack spacing={2}>
                <TextField
                  label="Display name"
                  value={draft.display_name}
                  onChange={(event) => setDraft({ ...draft, display_name: event.target.value })}
                />
                <TextField
                  label="Username"
                  value={draft.username}
                  onChange={(event) => setDraft({ ...draft, username: event.target.value })}
                />
                <TextField
                  label="Email"
                  value={draft.email}
                  onChange={(event) => setDraft({ ...draft, email: event.target.value })}
                />
              </Stack>

              <Box>
                <Typography fontWeight={600} sx={{ mb: 1 }}>
                  Account status
                </Typography>
                <Stack direction="row" alignItems="center" justifyContent="space-between">
                  <Box>
                    <Typography>{draft.status === 'active' ? 'Active' : 'Disabled'}</Typography>
                    <Typography variant="body2" color="text.secondary">
                      Disabled users cannot sign in to this identity service or connected applications.
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
              </Box>

              <Typography variant="caption" color="text.secondary">
                Created {formatDate(draft.created_at)}
              </Typography>
            </Stack>
          )}

          {tab === 1 && (
            <Stack spacing={3.5} sx={{ p: 3 }}>
              <Box>
                <Typography fontWeight={600}>Roles</Typography>
                <Typography variant="body2" color="text.secondary" sx={{ mb: 1.5 }}>
                  Roles are reusable authorization labels. Platform admin grants Auctor
                  administration.
                </Typography>
                <Stack direction="row" gap={1} flexWrap="wrap">
                  {roles.map((role) => {
                    const active = draft.roles.includes(role.name)
                    return (
                      <Chip
                        key={role.id}
                        label={role.name}
                        color={active ? 'primary' : 'default'}
                        variant={active ? 'filled' : 'outlined'}
                        onClick={() =>
                          setDraft({
                            ...draft,
                            roles: active
                              ? draft.roles.filter((name) => name !== role.name)
                              : [...draft.roles, role.name],
                          })
                        }
                      />
                    )
                  })}
                </Stack>
              </Box>

              <Box>
                <Typography fontWeight={600}>Groups</Typography>
                <Typography variant="body2" color="text.secondary" sx={{ mb: 1.5 }}>
                  Groups organize users independently from their identity and can later be emitted
                  as OIDC claims.
                </Typography>
                <Stack direction="row" gap={1} flexWrap="wrap">
                  {groups.map((group) => {
                    const active = draft.groups.includes(group.name)
                    return (
                      <Chip
                        key={group.id}
                        label={group.name}
                        color={active ? 'success' : 'default'}
                        variant={active ? 'filled' : 'outlined'}
                        onClick={() =>
                          setDraft({
                            ...draft,
                            groups: active
                              ? draft.groups.filter((name) => name !== group.name)
                              : [...draft.groups, group.name],
                          })
                        }
                      />
                    )
                  })}
                  {!groups.length && (
                    <Typography variant="body2" color="text.secondary">
                      No groups have been created.
                    </Typography>
                  )}
                </Stack>
              </Box>
            </Stack>
          )}

          {tab === 2 && (
            <Stack spacing={3.5} sx={{ p: 3 }}>
              <Box>
                <Stack direction="row" alignItems="center" justifyContent="space-between" mb={1}>
                  <Box>
                    <Typography fontWeight={600}>Sessions and devices</Typography>
                    <Typography variant="body2" color="text.secondary">
                      Inspect recent clients and revoke them individually.
                    </Typography>
                  </Box>
                  <Button
                    size="small"
                    color="error"
                    onClick={async () => {
                      const result = await api.revokeSessions(draft.id)
                      await loadSessions(draft.id)
                      await onChanged('Revoked ' + result.revoked + ' sessions')
                    }}
                  >
                    Revoke all
                  </Button>
                </Stack>

                <Stack spacing={1}>
                  {sessions.map((session) => (
                    <Box
                      key={session.id}
                      sx={{
                        border: '1px solid',
                        borderColor: 'divider',
                        borderRadius: 1.5,
                        p: 1.75,
                      }}
                    >
                      <Stack direction="row" spacing={1.5} alignItems="flex-start">
                        <Box
                          sx={{
                            width: 38,
                            height: 38,
                            borderRadius: 2.5,
                            bgcolor: '#eef3f8',
                            display: 'grid',
                            placeItems: 'center',
                            color: 'text.secondary',
                            flex: '0 0 auto',
                          }}
                        >
                          <DevicesRounded fontSize="small" />
                        </Box>
                        <Box sx={{ flex: 1, minWidth: 0 }}>
                          <Stack direction="row" spacing={1} alignItems="center">
                            <Typography fontWeight={600} variant="body2">
                              {sessionLabel(session.user_agent)}
                            </Typography>
                            <Chip
                              size="small"
                              label={session.active ? 'Active' : 'Ended'}
                              color={session.active ? 'success' : 'default'}
                              variant="outlined"
                            />
                          </Stack>
                          <Typography variant="caption" color="text.secondary" display="block">
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
                            {session.user_agent ?? 'No user-agent information'}
                          </Typography>
                        </Box>
                        {session.active && (
                          <Button
                            size="small"
                            color="error"
                            onClick={async () => {
                              await api.revokeSession(session.id)
                              await loadSessions(draft.id)
                              await onChanged('Session revoked')
                            }}
                          >
                            Revoke
                          </Button>
                        )}
                      </Stack>
                    </Box>
                  ))}
                  {!sessions.length && (
                    <Typography color="text.secondary" variant="body2" sx={{ py: 2 }}>
                      No sessions recorded for this user.
                    </Typography>
                  )}
                </Stack>
              </Box>

              <Divider />

              <Box>
                <Typography fontWeight={600}>Reset password</Typography>
                <Typography variant="body2" color="text.secondary" sx={{ mb: 1.5 }}>
                  Resetting the password immediately revokes all active sessions.
                </Typography>
                <Stack direction={{ xs: 'column', sm: 'row' }} spacing={1}>
                  <TextField
                    fullWidth
                    size="small"
                    type="password"
                    label="New password"
                    value={password}
                    onChange={(event) => setPassword(event.target.value)}
                  />
                  <Button
                    variant="outlined"
                    startIcon={<KeyRounded />}
                    disabled={password.length < 12}
                    onClick={async () => {
                      await api.setPassword(draft.id, password)
                      setPassword('')
                      await loadSessions(draft.id)
                      await onChanged('Password reset')
                    }}
                  >
                    Reset
                  </Button>
                </Stack>
              </Box>
            </Stack>
          )}
        </Box>

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
