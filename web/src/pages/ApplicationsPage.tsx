import { AddRounded, AppsRounded } from '@mui/icons-material'
import { Box, Button, Card, CardContent, Stack, Typography } from '@mui/material'

export function ApplicationsPage() {
  return (
    <Stack spacing={2.5}>
      <Stack direction={{ xs: 'column', sm: 'row' }} justifyContent="space-between" gap={2}>
        <Box>
          <Typography variant="h4">Applications</Typography>
          <Typography color="text.secondary" sx={{ mt: 0.5 }}>
            OAuth and OpenID Connect clients that trust Auctor.
          </Typography>
        </Box>
        <Button variant="contained" startIcon={<AddRounded />} disabled>
          Add application
        </Button>
      </Stack>
      <Card sx={{ borderRadius: 5 }}>
        <CardContent sx={{ py: 8, textAlign: 'center' }}>
          <AppsRounded sx={{ fontSize: 52, color: 'primary.main', mb: 2 }} />
          <Typography variant="h6">Application management is next</Typography>
          <Typography color="text.secondary" sx={{ maxWidth: 560, mx: 'auto', mt: 1 }}>
            The data model is already present. Client credentials, redirect URI policy, PKCE,
            consent, and device authorization will land with the OIDC provider implementation.
          </Typography>
        </CardContent>
      </Card>
    </Stack>
  )
}
