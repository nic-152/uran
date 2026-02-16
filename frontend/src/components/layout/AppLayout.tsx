import type { ReactNode } from 'react'
import Box from '@mui/material/Box'
import Toolbar from '@mui/material/Toolbar'
import Sidebar, { type AppSection, drawerWidth } from './Sidebar'
import Topbar from './Topbar'

interface AppLayoutProps {
  activeSection: AppSection
  onSectionChange: (section: AppSection) => void
  title: string
  subtitle?: string
  children: ReactNode
}

export default function AppLayout({
  activeSection,
  onSectionChange,
  title,
  subtitle,
  children,
}: AppLayoutProps) {
  return (
    <Box sx={{ display: 'flex', width: '100%', minHeight: '100vh' }}>
      <Sidebar activeSection={activeSection} onSectionChange={onSectionChange} />

      <Box
        component="main"
        sx={{
          flexGrow: 1,
          ml: `${drawerWidth}px`,
          minHeight: '100vh',
          bgcolor: 'background.default',
        }}
      >
        <Topbar title={title} subtitle={subtitle} />
        <Toolbar />
        <Box sx={{ p: 3 }}>{children}</Box>
      </Box>
    </Box>
  )
}
