import type { AuditEvent, Me, Role, Stats, User, UserList } from './types'

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
  me: () => request<Me>('/api/me'),
  login: (username: string, password: string) =>
    request<{ ok: boolean }>('/api/auth/login', {
      method: 'POST',
      body: JSON.stringify({ username, password }),
    }),
  logout: () => request<{ ok: boolean }>('/api/auth/logout', { method: 'POST' }),

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
  audit: () => request<{ items: AuditEvent[] }>('/api/admin/audit?limit=100'),
}
