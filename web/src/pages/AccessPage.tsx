import {
  AddRounded,
  DeleteOutlineRounded,
  EditRounded,
  GroupsRounded,
  ShieldRounded,
} from '@mui/icons-material'
import {
  Box,
  Button,
  Card,
  CardContent,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  IconButton,
  Stack,
  Tab,
  Tabs,
  TextField,
  Typography,
} from '@mui/material'
import { FormEvent, useCallback, useEffect, useState } from 'react'
import { api } from '../api'
import type { Group, Role } from '../types'

type Kind = 'groups' | 'roles'

type EditorState = {
  kind: Kind
  id?: string
  name: string
  description: string
}

export function AccessPage() {
  const [tab, setTab] = useState<Kind>('groups')
  const [groups, setGroups] = useState<Group[]>([])
  const [roles, setRoles] = useState<Role[]>([])
  const [editor, setEditor] = useState<EditorState | null>(null)
  const [deleteTarget, setDeleteTarget] = useState<EditorState | null>(null)

  const load = useCallback(async () => {
    const [groupResult, roleResult] = await Promise.all([api.groups(), api.roles()])
    setGroups(groupResult.items)
    setRoles(roleResult.items)
  }, [])

  useEffect(() => {
    load()
  }, [load])

  const items = tab === 'groups' ? groups : roles
  const isProtectedRole = (name: string) => name === 'user' || name === 'platform-admin'

  return (
    <Stack spacing={3}>
      <Stack direction={{ xs: 'column', sm: 'row' }} justifyContent="space-between" gap={2}>
        <Box>
          <Typography variant="h4">Access</Typography>
          <Typography color="text.secondary" sx={{ mt: 0.5 }}>
            Organize people with groups and assign reusable authorization roles.
          </Typography>
        </Box>
        <Button
          variant="contained"
          startIcon={<AddRounded />}
          onClick={() =>
            setEditor({
              kind: tab,
              name: '',
              description: '',
            })
          }
        >
          {tab === 'groups' ? 'Create group' : 'Create role'}
        </Button>
      </Stack>

      <Tabs value={tab} onChange={(_, value: Kind) => setTab(value)} sx={{ minHeight: 44 }}>
        <Tab value="groups" label="Groups" />
        <Tab value="roles" label="Roles" />
      </Tabs>

      <Stack spacing={1.5}>
        {items.map((item) => {
          const isGroup = tab === 'groups'
          const count = isGroup ? (item as Group).member_count : (item as Role).user_count
          const protectedRole = !isGroup && isProtectedRole(item.name)
          return (
            <Card key={item.id} sx={{ borderRadius: 1.5 }}>
              <CardContent sx={{ py: 2.25, '&:last-child': { pb: 2.25 } }}>
                <Stack direction="row" alignItems="center" spacing={2}>
                  <Box
                    sx={{
                      width: 44,
                      height: 44,
                      borderRadius: 1.5,
                      bgcolor: 'action.hover',
                      color: isGroup ? 'success.main' : 'primary.main',
                      display: 'grid',
                      placeItems: 'center',
                      flex: '0 0 auto',
                    }}
                  >
                    {isGroup ? <GroupsRounded /> : <ShieldRounded />}
                  </Box>

                  <Box sx={{ minWidth: 0, flex: 1 }}>
                    <Stack direction="row" alignItems="center" spacing={1}>
                      <Typography fontWeight={600}>{item.name}</Typography>
                      {protectedRole && (
                        <Typography
                          variant="caption"
                          sx={{
                            bgcolor: 'action.hover',
                            color: 'text.secondary',
                            px: 1,
                            py: 0.25,
                            borderRadius: 1,
                          }}
                        >
                          built-in
                        </Typography>
                      )}
                    </Stack>
                    <Typography variant="body2" color="text.secondary" noWrap>
                      {item.description || 'No description'}
                    </Typography>
                  </Box>

                  <Typography
                    variant="body2"
                    color="text.secondary"
                    sx={{ display: { xs: 'none', sm: 'block' }, minWidth: 92, textAlign: 'right' }}
                  >
                    {count} {isGroup ? (count === 1 ? 'member' : 'members') : count === 1 ? 'user' : 'users'}
                  </Typography>

                  <IconButton
                    aria-label={'Edit ' + item.name}
                    onClick={() =>
                      setEditor({
                        kind: tab,
                        id: item.id,
                        name: item.name,
                        description: item.description,
                      })
                    }
                  >
                    <EditRounded fontSize="small" />
                  </IconButton>
                  <IconButton
                    aria-label={'Delete ' + item.name}
                    disabled={protectedRole}
                    onClick={() =>
                      setDeleteTarget({
                        kind: tab,
                        id: item.id,
                        name: item.name,
                        description: item.description,
                      })
                    }
                  >
                    <DeleteOutlineRounded fontSize="small" />
                  </IconButton>
                </Stack>
              </CardContent>
            </Card>
          )
        })}

        {!items.length && (
          <Card sx={{ borderRadius: 2 }}>
            <CardContent sx={{ py: 7, textAlign: 'center' }}>
              <Typography variant="h6">
                No {tab === 'groups' ? 'groups' : 'roles'} yet
              </Typography>
              <Typography color="text.secondary" sx={{ mt: 0.75 }}>
                {tab === 'groups'
                  ? 'Create a group to organize users without changing their global identity.'
                  : 'Create a reusable role for application and infrastructure authorization.'}
              </Typography>
            </CardContent>
          </Card>
        )}
      </Stack>

      <AccessEditor
        value={editor}
        onClose={() => setEditor(null)}
        onSaved={async () => {
          setEditor(null)
          await load()
        }}
      />

      <Dialog open={Boolean(deleteTarget)} onClose={() => setDeleteTarget(null)} maxWidth="xs" fullWidth>
        <DialogTitle>Delete {deleteTarget?.kind === 'groups' ? 'group' : 'role'}?</DialogTitle>
        <DialogContent>
          <Typography color="text.secondary">
            {deleteTarget?.name} will be removed from all users. User accounts themselves are not
            deleted.
          </Typography>
        </DialogContent>
        <DialogActions sx={{ p: 2.5 }}>
          <Button onClick={() => setDeleteTarget(null)}>Cancel</Button>
          <Button
            color="error"
            variant="contained"
            onClick={async () => {
              if (!deleteTarget?.id) return
              if (deleteTarget.kind === 'groups') {
                await api.deleteGroup(deleteTarget.id)
              } else {
                await api.deleteRole(deleteTarget.id)
              }
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

function AccessEditor({
  value,
  onClose,
  onSaved,
}: {
  value: EditorState | null
  onClose: () => void
  onSaved: () => Promise<void>
}) {
  const [draft, setDraft] = useState<EditorState | null>(value)
  const [busy, setBusy] = useState(false)

  useEffect(() => setDraft(value), [value])
  if (!draft) return null

  const editing = Boolean(draft.id)
  const label = draft.kind === 'groups' ? 'group' : 'role'

  async function submit(event: FormEvent) {
    event.preventDefault()
    if (!draft) return
    setBusy(true)
    try {
      if (draft.kind === 'groups') {
        if (draft.id) {
          await api.updateGroup(draft.id, {
            name: draft.name,
            description: draft.description,
          })
        } else {
          await api.createGroup({ name: draft.name, description: draft.description })
        }
      } else if (draft.id) {
        await api.updateRole(draft.id, {
          name: draft.name,
          description: draft.description,
        })
      } else {
        await api.createRole({ name: draft.name, description: draft.description })
      }
      await onSaved()
    } finally {
      setBusy(false)
    }
  }

  return (
    <Dialog open={Boolean(value)} onClose={onClose} maxWidth="sm" fullWidth>
      <Box component="form" onSubmit={submit}>
        <DialogTitle>
          {editing ? 'Edit ' : 'Create '}
          {label}
        </DialogTitle>
        <DialogContent>
          <Stack spacing={2} sx={{ pt: 1 }}>
            <TextField
              label="Name"
              value={draft.name}
              onChange={(event) => setDraft({ ...draft, name: event.target.value })}
              required
              autoFocus
            />
            <TextField
              label="Description"
              value={draft.description}
              onChange={(event) => setDraft({ ...draft, description: event.target.value })}
              multiline
              minRows={3}
            />
          </Stack>
        </DialogContent>
        <DialogActions sx={{ p: 2.5 }}>
          <Button onClick={onClose}>Cancel</Button>
          <Button type="submit" variant="contained" disabled={busy || !draft.name.trim()}>
            {editing ? 'Save' : 'Create'}
          </Button>
        </DialogActions>
      </Box>
    </Dialog>
  )
}
