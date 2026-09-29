import { Avatar } from '@mui/material'
import { md5 } from 'js-md5'
import { useEffect, useMemo, useState } from 'react'
import { useBrand } from './Branding'

type Props = {
  userId: string
  email: string
  name: string
  size?: number
}

function directAvatarUrl(template: string, email: string, size: number) {
  const normalized = email.trim().toLowerCase()
  return template
    .replaceAll('{email_md5}', md5(normalized))
    .replaceAll('{email}', encodeURIComponent(normalized))
    .replaceAll('{size}', String(size))
}

export function UserAvatar({ userId, email, name, size = 40 }: Props) {
  const { brand } = useBrand()
  const [failed, setFailed] = useState(false)
  const sourceSize = Math.max(size * 2, 80)

  const source = useMemo(
    () =>
      brand.avatar_delivery === 'proxy'
        ? '/api/avatar/user/' + userId + '?size=' + sourceSize
        : directAvatarUrl(brand.avatar_source_template, email, sourceSize),
    [brand.avatar_delivery, brand.avatar_source_template, email, sourceSize, userId],
  )

  useEffect(() => setFailed(false), [source])

  const initials = name
    .trim()
    .split(/\s+/)
    .slice(0, 2)
    .map((part) => part[0]?.toUpperCase() ?? '')
    .join('')

  return (
    <Avatar
      src={failed ? undefined : source}
      onError={() => setFailed(true)}
      sx={{
        width: size,
        height: size,
        bgcolor: 'primary.light',
        color: 'primary.main',
        fontWeight: 600,
        fontSize: size * 0.36,
      }}
    >
      {initials || '?'}
    </Avatar>
  )
}
