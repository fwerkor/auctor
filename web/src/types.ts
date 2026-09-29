export type Me = {
  id: string
  username: string
  email: string
  display_name: string
  roles: string[]
}

export type User = {
  id: string
  username: string
  email: string
  display_name: string
  status: 'active' | 'disabled'
  created_at: string
  updated_at: string
  roles: string[]
  active_sessions: number
  last_seen_at: string | null
}

export type UserList = {
  items: User[]
  total: number
}

export type Role = {
  id: string
  name: string
  description: string
}

export type AuditEvent = {
  id: number
  actor_user_id: string | null
  actor_username: string | null
  action: string
  target_type: string
  target_id: string | null
  metadata: Record<string, unknown>
  created_at: string
}

export type Stats = {
  users_total: number
  users_active: number
  sessions_active: number
  applications_total: number
}
