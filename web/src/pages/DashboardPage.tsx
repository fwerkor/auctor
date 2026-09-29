import {
  AppsRounded,
  GroupsRounded,
  PersonRounded,
  VpnKeyRounded,
} from '@mui/icons-material'
import { Box, Card, CardContent, Grid, Skeleton, Stack, Typography } from '@mui/material'
import { useEffect, useState } from 'react'
import { api } from '../api'
import type { Stats } from '../types'

const cards = [
  { key: 'users_total', label: 'Total users', icon: PersonRounded },
  { key: 'users_active', label: 'Active users', icon: GroupsRounded },
  { key: 'sessions_active', label: 'Active sessions', icon: VpnKeyRounded },
  { key: 'applications_total', label: 'Applications', icon: AppsRounded },
] as const

export function DashboardPage() {
  const [stats, setStats] = useState<Stats | null>(null)

  useEffect(() => {
    api.stats().then(setStats)
  }, [])

  return (
    <Stack spacing={3}>
      <Box>
        <Typography variant="h4">Overview</Typography>
        <Typography color="text.secondary" sx={{ mt: 0.5 }}>
          Identity, access, and account activity at a glance.
        </Typography>
      </Box>

      <Grid container spacing={2}>
        {cards.map(({ key, label, icon: Icon }) => (
          <Grid key={key} size={{ xs: 12, sm: 6, xl: 3 }}>
            <Card sx={{ height: '100%', borderRadius: 5 }}>
              <CardContent sx={{ p: 3 }}>
                <Stack direction="row" justifyContent="space-between" alignItems="flex-start">
                  <Box>
                    <Typography color="text.secondary" variant="body2">
                      {label}
                    </Typography>
                    <Typography variant="h4" sx={{ mt: 1.2 }}>
                      {stats ? stats[key].toLocaleString() : <Skeleton width={72} />}
                    </Typography>
                  </Box>
                  <Box
                    sx={{
                      width: 44,
                      height: 44,
                      borderRadius: 3,
                      bgcolor: '#e8f0fe',
                      color: '#0b57d0',
                      display: 'grid',
                      placeItems: 'center',
                    }}
                  >
                    <Icon />
                  </Box>
                </Stack>
              </CardContent>
            </Card>
          </Grid>
        ))}
      </Grid>

      <Card sx={{ borderRadius: 5 }}>
        <CardContent sx={{ p: 3 }}>
          <Typography variant="h6">Security posture</Typography>
          <Typography color="text.secondary" sx={{ mt: 0.75 }}>
            Auctor keeps administrator privileges as roles on ordinary accounts. Sessions can be
            revoked centrally, and all privileged user changes are written to the audit log.
          </Typography>
        </CardContent>
      </Card>
    </Stack>
  )
}
