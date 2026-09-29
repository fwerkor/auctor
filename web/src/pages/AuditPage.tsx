import {
  Box,
  Chip,
  Stack,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Typography,
} from '@mui/material'
import { useEffect, useState } from 'react'
import { api } from '../api'
import type { AuditEvent } from '../types'

export function AuditPage() {
  const [items, setItems] = useState<AuditEvent[]>([])

  useEffect(() => {
    api.audit().then((result) => setItems(result.items))
  }, [])

  return (
    <Stack spacing={2.5}>
      <Box>
        <Typography variant="h4">Audit log</Typography>
        <Typography color="text.secondary" sx={{ mt: 0.5 }}>
          Immutable records of privileged account changes.
        </Typography>
      </Box>
      <TableContainer
        sx={{
          bgcolor: 'background.paper',
          border: '1px solid',
          borderColor: 'divider',
          borderRadius: 2,
          overflow: 'hidden',
        }}
      >
        <Table>
          <TableHead>
            <TableRow sx={{ bgcolor: '#f8fafd' }}>
              <TableCell>Time</TableCell>
              <TableCell>Actor</TableCell>
              <TableCell>Action</TableCell>
              <TableCell>Target</TableCell>
            </TableRow>
          </TableHead>
          <TableBody>
            {items.map((event) => (
              <TableRow key={event.id}>
                <TableCell>
                  {new Intl.DateTimeFormat(undefined, {
                    dateStyle: 'medium',
                    timeStyle: 'short',
                  }).format(new Date(event.created_at))}
                </TableCell>
                <TableCell>{event.actor_username ?? 'system'}</TableCell>
                <TableCell>
                  <Chip label={event.action} size="small" variant="outlined" />
                </TableCell>
                <TableCell>
                  <Typography variant="body2">
                    {event.target_type}
                    {event.target_id ? ' · ' + event.target_id : ''}
                  </Typography>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </TableContainer>
    </Stack>
  )
}
