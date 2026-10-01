export type Branding = {
  site_name: string
  site_url: string
  logo_url: string
  avatar_source_template: string
  avatar_delivery: 'direct' | 'proxy'
}

export type AvatarSettings = {
  avatar_source_template: string
  avatar_delivery: 'direct' | 'proxy'
}

export type ReservedUsername = {
  username: string
  note: string
  created_at: string
}

export type Me = {
  id: string
  session_id: string
  username: string
  email: string
  display_name: string
  roles: string[]
  groups: string[]
}

export type User = {
  id: string
  username: string
  email: string
  display_name: string
  status: 'active' | 'disabled' | 'pending_email'
  created_at: string
  updated_at: string
  roles: string[]
  groups: string[]
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
  user_count: number
  created_at: string
}

export type Group = {
  id: string
  name: string
  description: string
  member_count: number
  created_at: string
  updated_at: string
}

export type Application = {
  id: string
  client_id: string
  name: string
  app_type: 'web' | 'native' | 'service'
  redirect_uris: string[]
  status: 'active' | 'disabled'
  created_at: string
  updated_at: string
}

export type Session = {
  id: string
  created_at: string
  expires_at: string
  last_seen_at: string
  ip: string | null
  user_agent: string | null
  revoked_at: string | null
  active: boolean
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

export type RegistrationConfig = {
  enabled: boolean
  require_email_verification: boolean
}

export type ExternalProvider = {
  id: string
  name: string
}

export type ExternalIdentity = {
  provider: string
  subject: string
  login: string
  display_name: string | null
  avatar_url: string | null
  profile_url: string | null
  created_at: string
  updated_at: string
}

export type AuthSettings = {
  registration_enabled: boolean
  registration_require_email_verification: boolean
  smtp_host: string
  smtp_port: number
  smtp_security: 'starttls' | 'tls' | 'none'
  smtp_username: string
  smtp_from_email: string
  smtp_from_name: string
  smtp_password_configured: boolean
  github_oauth_enabled: boolean
  github_oauth_client_id: string
  github_oauth_client_secret_configured: boolean
}
