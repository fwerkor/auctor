import type { Application, AuditEvent, AvatarSettings, Branding, Group, Me, ReservedUsername, Role, Session, Stats, User, UserList } from './types'

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(path, {
    credentials: 'same-origin',
    ...init,
    headers: {
      'Content-Type': 'application/json',
      ...(init?.headers ?? {}),
    },
  })

  if (!response.ok) {
    const error = new Error('Request failed')
    Object.assign(error, { status: response.status })
    throw error
  }

  return response.json() as Promise<T>
}

export const api = {
  config: () => request<Branding>('/api/config'),
  updateBranding: (payload: Pick<Branding, 'site_name' | 'site_url' | 'logo_url'>) =>
    request<Branding>('/api/admin/branding', {
      method: 'PUT',
      body: JSON.stringify(payload),
    }),
  avatarSettings: () => request<AvatarSettings>('/api/admin/avatar-settings'),
  updateAvatarSettings: (payload: AvatarSettings) =>
    request<AvatarSettings>('/api/admin/avatar-settings', {
      method: 'PUT',
      body: JSON.stringify(payload),
    }),
  reservedUsernames: () =>
    request<{ items: ReservedUsername[] }>('/api/admin/reserved-usernames'),
  addReservedUsername: (username: string, note = '') =>
    request<{ username: string }>('/api/admin/reserved-usernames', {
      method: 'POST',
      body: JSON.stringify({ username, note }),
    }),
  deleteReservedUsername: (username: string) =>
    request<{ ok: boolean }>('/api/admin/reserved-usernames/' + encodeURIComponent(username), {
      method: 'DELETE',
    }),
  setupStatus: () => request<{ required: boolean }>('/api/setup/status'),
  setup: (payload: {
    setup_token: string
    username: string
    email: string
    display_name: string
    password: string
  }) =>
    request<{ id: string }>('/api/setup', {
      method: 'POST',
      body: JSON.stringify(payload),
    }),
  me: () => request<Me>('/api/me'),
  login: (username: string, password: string) =>
    request<{ ok: boolean }>('/api/auth/login', {
      method: 'POST',
      body: JSON.stringify({ username, password }),
    }),
  logout: () => request<{ ok: boolean }>('/api/auth/logout', { method: 'POST' }),
  updateProfile: (displayName: string) =>
    request<{ ok: boolean }>('/api/account/profile', {
      method: 'PATCH',
      body: JSON.stringify({ display_name: displayName }),
    }),
  changePassword: (currentPassword: string, newPassword: string) =>
    request<{ ok: boolean; sessions_revoked: boolean }>('/api/account/password', {
      method: 'PUT',
      body: JSON.stringify({ current_password: currentPassword, new_password: newPassword }),
    }),
  ownSessions: () => request<{ items: Session[] }>('/api/account/sessions'),
  revokeOwnSession: (sessionId: string) =>
    request<{ ok: boolean }>('/api/account/sessions/' + sessionId, { method: 'DELETE' }),

  stats: () => request<Stats>('/api/admin/stats'),
  users: (q = '', status = '') => {
    const params = new URLSearchParams({ q, status, limit: '100' })
    return request<UserList>('/api/admin/users?' + params.toString())
  },
  user: (id: string) => request<User>('/api/admin/users/' + id),
  createUser: (payload: {
    username: string
    email: string
    display_name: string
    password: string
    roles: string[]
  }) =>
    request<{ id: string }>('/api/admin/users', {
      method: 'POST',
      body: JSON.stringify(payload),
    }),
  updateUser: (id: string, payload: Partial<Pick<User, 'username' | 'email' | 'display_name' | 'status'>>) =>
    request<{ ok: boolean }>('/api/admin/users/' + id, {
      method: 'PATCH',
      body: JSON.stringify(payload),
    }),
  setRoles: (id: string, roles: string[]) =>
    request<{ ok: boolean }>('/api/admin/users/' + id + '/roles', {
      method: 'PUT',
      body: JSON.stringify({ roles }),
    }),
  setPassword: (id: string, password: string) =>
    request<{ ok: boolean }>('/api/admin/users/' + id + '/password', {
      method: 'PUT',
      body: JSON.stringify({ password }),
    }),
  revokeSessions: (id: string) =>
    request<{ ok: boolean; revoked: number }>('/api/admin/users/' + id + '/revoke-sessions', {
      method: 'POST',
    }),
  roles: () => request<{ items: Role[] }>('/api/admin/roles'),
  createRole: (payload: { name: string; description: string }) =>
    request<{ id: string }>('/api/admin/roles', {
      method: 'POST',
      body: JSON.stringify(payload),
    }),
  updateRole: (id: string, payload: { name?: string; description?: string }) =>
    request<{ ok: boolean }>('/api/admin/roles/' + id, {
      method: 'PATCH',
      body: JSON.stringify(payload),
    }),
  deleteRole: (id: string) =>
    request<{ ok: boolean }>('/api/admin/roles/' + id, { method: 'DELETE' }),

  groups: () => request<{ items: Group[] }>('/api/admin/groups'),
  createGroup: (payload: { name: string; description: string }) =>
    request<{ id: string }>('/api/admin/groups', {
      method: 'POST',
      body: JSON.stringify(payload),
    }),
  updateGroup: (id: string, payload: { name?: string; description?: string }) =>
    request<{ ok: boolean }>('/api/admin/groups/' + id, {
      method: 'PATCH',
      body: JSON.stringify(payload),
    }),
  deleteGroup: (id: string) =>
    request<{ ok: boolean }>('/api/admin/groups/' + id, { method: 'DELETE' }),
  setGroups: (id: string, groupIds: string[]) =>
    request<{ ok: boolean }>('/api/admin/users/' + id + '/groups', {
      method: 'PUT',
      body: JSON.stringify({ group_ids: groupIds }),
    }),

  sessions: (userId: string) =>
    request<{ items: Session[] }>('/api/admin/users/' + userId + '/sessions'),
  revokeSession: (sessionId: string) =>
    request<{ ok: boolean }>('/api/admin/sessions/' + sessionId, { method: 'DELETE' }),
  bulkUsers: (ids: string[], action: 'activate' | 'disable' | 'revoke_sessions') =>
    request<{ ok: boolean; affected: number }>('/api/admin/users/bulk', {
      method: 'POST',
      body: JSON.stringify({ ids, action }),
    }),

  applications: () => request<{ items: Application[] }>('/api/admin/applications'),
  createApplication: (payload: {
    name: string
    client_id?: string
    app_type: 'web' | 'native' | 'service'
    redirect_uris: string[]
  }) =>
    request<{ id: string; client_id: string }>('/api/admin/applications', {
      method: 'POST',
      body: JSON.stringify(payload),
    }),
  updateApplication: (
    id: string,
    payload: Partial<Pick<Application, 'name' | 'client_id' | 'app_type' | 'redirect_uris' | 'status'>>,
  ) =>
    request<{ ok: boolean }>('/api/admin/applications/' + id, {
      method: 'PATCH',
      body: JSON.stringify(payload),
    }),
  deleteApplication: (id: string) =>
    request<{ ok: boolean }>('/api/admin/applications/' + id, { method: 'DELETE' }),

  audit: () => request<{ items: AuditEvent[] }>('/api/admin/audit?limit=100'),
}
