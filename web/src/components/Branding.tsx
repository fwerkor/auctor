import { Box } from '@mui/material'
import { createContext, useContext, useEffect, useMemo, useState } from 'react'
import { api } from '../api'
import type { Branding } from '../types'

const fallback: Branding = {
  site_name: 'Auctor',
  site_url: '',
  logo_url: '/brand/auctor.svg',
}

const BrandContext = createContext<{
  brand: Branding
  refresh: () => Promise<void>
}>({
  brand: fallback,
  refresh: async () => {},
})

export function BrandProvider({ children }: { children: React.ReactNode }) {
  const [brand, setBrand] = useState<Branding>(fallback)

  async function refresh() {
    try {
      const value = await api.config()
      setBrand({
        site_name: value.site_name || fallback.site_name,
        site_url: value.site_url,
        logo_url: value.logo_url || fallback.logo_url,
      })
    } catch {
      setBrand(fallback)
    }
  }

  useEffect(() => {
    refresh()
  }, [])

  useEffect(() => {
    document.title = brand.site_name
    const favicon = document.querySelector<HTMLLinkElement>('link[rel="icon"]')
    if (favicon) favicon.href = brand.logo_url || fallback.logo_url
  }, [brand])

  const value = useMemo(() => ({ brand, refresh }), [brand])
  return <BrandContext.Provider value={value}>{children}</BrandContext.Provider>
}

export function useBrand() {
  return useContext(BrandContext)
}

export function BrandMark({
  size = 40,
  logoUrl,
}: {
  size?: number
  logoUrl?: string
}) {
  const { brand } = useBrand()
  return (
    <Box
      component="img"
      src={logoUrl || brand.logo_url || fallback.logo_url}
      alt=""
      sx={{
        width: size,
        height: size,
        display: 'block',
        objectFit: 'contain',
        flex: '0 0 auto',
      }}
    />
  )
}
