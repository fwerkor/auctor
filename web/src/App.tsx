import {
  AccountCircleRounded,
  AdminPanelSettingsRounded,
  AppsRounded,
  DashboardRounded,
  LogoutRounded,
  MenuRounded,
  PeopleRounded,
  PolicyRounded,
  PaletteRounded,
  SearchRounded,
} from '@mui/icons-material'
import {
  AppBar,
  Box,
  CircularProgress,
  Divider,
  Drawer,
  IconButton,
  InputBase,
  List,
  ListItemButton,
  ListItemIcon,
  ListItemText,
  Stack,
  Toolbar,
  Typography,
  useMediaQuery,
} from '@mui/material'
import { useTheme } from '@mui/material/styles'
import { Suspense, lazy, useCallback, useEffect, useState } from 'react'
import { Navigate, Route, Routes, useLocation, useNavigate } from 'react-router-dom'
import { api } from './api'
import { BrandMark, useBrand } from './components/Branding'
import { UserAvatar } from './components/UserAvatar'
import { LoginPage } from './pages/LoginPage'
import { SetupPage } from './pages/SetupPage'
import type { Me } from './types'

const drawerWidth = 250

const AccountPage = lazy(() =>
  import('./pages/AccountPage').then((module) => ({ default: module.AccountPage })),
)
const DashboardPage = lazy(() =>
  import('./pages/DashboardPage').then((module) => ({ default: module.DashboardPage })),
)
const UsersPage = lazy(() =>
  import('./pages/UsersPage').then((module) => ({ default: module.UsersPage })),
)
const AccessPage = lazy(() =>
  import('./pages/AccessPage').then((module) => ({ default: module.AccessPage })),
)
const ApplicationsPage = lazy(() =>
  import('./pages/ApplicationsPage').then((module) => ({ default: module.ApplicationsPage })),
)
const AuditPage = lazy(() =>
  import('./pages/AuditPage').then((module) => ({ default: module.AuditPage })),
)
const BrandingPage = lazy(() =>
  import('./pages/BrandingPage').then((module) => ({ default: module.BrandingPage })),
)

const nav = [
  { path: '/', label: 'Overview', icon: DashboardRounded },
  { path: '/users', label: 'Users', icon: PeopleRounded },
  { path: '/access', label: 'Access', icon: AdminPanelSettingsRounded },
  { path: '/applications', label: 'Applications', icon: AppsRounded },
  { path: '/audit', label: 'Audit log', icon: PolicyRounded },
  { path: '/branding', label: 'Branding', icon: PaletteRounded },
  { path: '/account', label: 'My account', icon: AccountCircleRounded },
]

export default function App() {
  const [me, setMe] = useState<Me | null | undefined>(undefined)
  const [setupRequired, setSetupRequired] = useState<boolean | undefined>(undefined)

  const refreshMe = useCallback(async () => {
    try {
      setMe(await api.me())
    } catch {
      setMe(null)
    }
  }, [])

  const refreshSetup = useCallback(async () => {
    try {
      const result = await api.setupStatus()
      setSetupRequired(result.required)
      if (!result.required) await refreshMe()
    } catch {
      setSetupRequired(false)
      await refreshMe()
    }
  }, [refreshMe])

  useEffect(() => {
    refreshSetup()
  }, [refreshSetup])

  if (setupRequired === undefined || (!setupRequired && me === undefined)) {
    return (
      <Box sx={{ minHeight: '100vh', display: 'grid', placeItems: 'center' }}>
        <CircularProgress />
      </Box>
    )
  }

  if (setupRequired) return <SetupPage onComplete={refreshSetup} />
  if (!me) return <LoginPage onAuthenticated={refreshMe} />
  if (!me.roles.includes('platform-admin')) {
    return (
      <Suspense
        fallback={
          <Box sx={{ minHeight: '100vh', display: 'grid', placeItems: 'center' }}>
            <CircularProgress />
          </Box>
        }
      >
        <AccountPage
          me={me}
          standalone
          onRefresh={refreshMe}
          onSignedOut={() => setMe(null)}
        />
      </Suspense>
    )
  }

  return (
    <AdminShell
      me={me}
      onLogout={() => setMe(null)}
      onRefresh={refreshMe}
    />
  )
}

function AdminShell({
  me,
  onLogout,
  onRefresh,
}: {
  me: Me
  onLogout: () => void
  onRefresh: () => Promise<void>
}) {
  const { brand } = useBrand()
  const theme = useTheme()
  const mobile = useMediaQuery(theme.breakpoints.down('md'))
  const [drawerOpen, setDrawerOpen] = useState(false)
  const location = useLocation()
  const navigate = useNavigate()

  const drawer = (
    <Stack sx={{ height: '100%', p: 1.5 }}>
      <Stack direction="row" alignItems="center" spacing={1.4} sx={{ px: 1.5, height: 64 }}>
        <BrandMark size={36} />
        <Box>
          <Typography fontWeight={600} lineHeight={1.1}>
            {brand.site_name}
          </Typography>
          <Typography variant="caption" color="text.secondary">
            Administration
          </Typography>
        </Box>
      </Stack>
      <List sx={{ mt: 1 }}>
        {nav.map(({ path, label, icon: Icon }) => (
          <ListItemButton
            key={path}
            selected={location.pathname === path}
            onClick={() => {
              navigate(path)
              setDrawerOpen(false)
            }}
            sx={{
              borderRadius: 1.5,
              mb: 0.5,
              minHeight: 48,
              '&.Mui-selected': {
                bgcolor: '#d3e3fd',
                color: '#0842a0',
              },
              '&.Mui-selected:hover': { bgcolor: '#c6dafc' },
            }}
          >
            <ListItemIcon sx={{ minWidth: 42, color: 'inherit' }}>
              <Icon fontSize="small" />
            </ListItemIcon>
            <ListItemText primary={label} />
          </ListItemButton>
        ))}
      </List>
      <Box sx={{ flex: 1 }} />
      <Divider sx={{ mb: 1 }} />
      <ListItemButton
        sx={{ borderRadius: 1.5 }}
        onClick={async () => {
          await api.logout()
          onLogout()
        }}
      >
        <ListItemIcon sx={{ minWidth: 42 }}>
          <LogoutRounded fontSize="small" />
        </ListItemIcon>
        <ListItemText primary="Sign out" />
      </ListItemButton>
    </Stack>
  )

  return (
    <Box sx={{ minHeight: '100vh', bgcolor: 'background.default' }}>
      <AppBar
        position="fixed"
        elevation={0}
        color="transparent"
        sx={{
          bgcolor: 'rgba(248,250,253,.92)',
          backdropFilter: 'blur(18px)',
          borderBottom: { xs: '1px solid #e3e7ec', md: 'none' },
          width: { md: 'calc(100% - ' + drawerWidth + 'px)' },
          ml: { md: drawerWidth + 'px' },
        }}
      >
        <Toolbar sx={{ gap: 2, minHeight: 72 }}>
          {mobile && (
            <IconButton onClick={() => setDrawerOpen(true)}>
              <MenuRounded />
            </IconButton>
          )}
          <Box
            sx={{
              flex: 1,
              maxWidth: 720,
              height: 48,
              bgcolor: '#eef3f8',
              borderRadius: 2,
              display: { xs: 'none', sm: 'flex' },
              alignItems: 'center',
              px: 2,
              color: 'text.secondary',
            }}
          >
            <SearchRounded fontSize="small" />
            <InputBase
              placeholder={'Search ' + brand.site_name}
              sx={{ ml: 1.2, flex: 1 }}
              inputProps={{ 'aria-label': 'Search ' + brand.site_name }}
            />
          </Box>
          <Box sx={{ flex: { xs: 1, sm: 0 } }} />
          <Stack direction="row" spacing={1.2} alignItems="center">
            <Box sx={{ textAlign: 'right', display: { xs: 'none', sm: 'block' } }}>
              <Typography variant="body2" fontWeight={600}>
                {me.display_name}
              </Typography>
              <Typography variant="caption" color="text.secondary">
                Platform admin
              </Typography>
            </Box>
            <UserAvatar userId={me.id} name={me.display_name} size={36} />
          </Stack>
        </Toolbar>
      </AppBar>

      {mobile ? (
        <Drawer
          open={drawerOpen}
          onClose={() => setDrawerOpen(false)}
          slotProps={{ paper: { sx: { width: drawerWidth } } }}
        >
          {drawer}
        </Drawer>
      ) : (
        <Drawer
          variant="permanent"
          slotProps={{
            paper: {
              sx: {
                width: drawerWidth,
                borderRight: '1px solid #e3e7ec',
                bgcolor: '#f8fafd',
              },
            },
          }}
        >
          {drawer}
        </Drawer>
      )}

      <Box
        component="main"
        sx={{
          ml: { md: drawerWidth + 'px' },
          pt: '72px',
          minHeight: '100vh',
        }}
      >
        <Box sx={{ p: { xs: 2, sm: 3, lg: 4 }, maxWidth: 1500, mx: 'auto' }}>
          <Suspense
            fallback={
              <Box sx={{ minHeight: 320, display: 'grid', placeItems: 'center' }}>
                <CircularProgress />
              </Box>
            }
          >
            <Routes>
              <Route path="/" element={<DashboardPage />} />
              <Route path="/users" element={<UsersPage />} />
              <Route path="/access" element={<AccessPage />} />
              <Route path="/applications" element={<ApplicationsPage />} />
              <Route path="/audit" element={<AuditPage />} />
              <Route path="/branding" element={<BrandingPage />} />
              <Route
                path="/account"
                element={
                  <AccountPage
                    me={me}
                    onRefresh={onRefresh}
                    onSignedOut={onLogout}
                  />
                }
              />
              <Route path="*" element={<Navigate to="/" replace />} />
            </Routes>
          </Suspense>
        </Box>
      </Box>
    </Box>
  )
}
