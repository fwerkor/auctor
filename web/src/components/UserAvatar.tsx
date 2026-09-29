import { Avatar } from '@mui/material'
import { useState } from 'react'

type Props = {
  userId: string
  name: string
  size?: number
}

export function UserAvatar({ userId, name, size = 40 }: Props) {
  const [failed, setFailed] = useState(false)
  const initials = name
    .trim()
    .split(/\s+/)
    .slice(0, 2)
    .map((part) => part[0]?.toUpperCase() ?? '')
    .join('')

  return (
    <Avatar
      src={failed ? undefined : '/api/avatar/user/' + userId + '?size=' + Math.max(size * 2, 80)}
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
