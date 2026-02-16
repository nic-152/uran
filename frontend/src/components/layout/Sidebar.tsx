import type { ReactElement } from 'react'
import DashboardOutlined from '@mui/icons-material/DashboardOutlined'
import AssessmentOutlined from '@mui/icons-material/AssessmentOutlined'
import LibraryBooksOutlined from '@mui/icons-material/LibraryBooksOutlined'
import ContentPasteSearchOutlined from '@mui/icons-material/ContentPasteSearchOutlined'
import InsightsOutlined from '@mui/icons-material/InsightsOutlined'
import AdminPanelSettingsOutlined from '@mui/icons-material/AdminPanelSettingsOutlined'
import Box from '@mui/material/Box'
import Drawer from '@mui/material/Drawer'
import List from '@mui/material/List'
import ListItemButton from '@mui/material/ListItemButton'
import ListItemIcon from '@mui/material/ListItemIcon'
import ListItemText from '@mui/material/ListItemText'
import Toolbar from '@mui/material/Toolbar'
import Typography from '@mui/material/Typography'

export const drawerWidth = 248

export type AppSection = 'dashboard' | 'runs' | 'test_library' | 'templates' | 'analytics' | 'admin'

interface SidebarProps {
  activeSection: AppSection
  onSectionChange: (section: AppSection) => void
}

const items: Array<{ id: AppSection; label: string; icon: ReactElement }> = [
  { id: 'dashboard', label: 'Dashboard', icon: <DashboardOutlined /> },
  { id: 'runs', label: 'Runs', icon: <ContentPasteSearchOutlined /> },
  { id: 'test_library', label: 'Test Library', icon: <LibraryBooksOutlined /> },
  { id: 'templates', label: 'Templates', icon: <AssessmentOutlined /> },
  { id: 'analytics', label: 'Analytics', icon: <InsightsOutlined /> },
  { id: 'admin', label: 'Admin', icon: <AdminPanelSettingsOutlined /> },
]

export default function Sidebar({ activeSection, onSectionChange }: SidebarProps) {
  return (
    <Drawer
      variant="permanent"
      sx={{
        width: drawerWidth,
        flexShrink: 0,
        '& .MuiDrawer-paper': {
          width: drawerWidth,
          boxSizing: 'border-box',
          borderRight: '1px solid',
          borderColor: 'divider',
        },
      }}
    >
      <Toolbar>
        <Box>
          <Typography variant="h6" fontWeight={700}>
            Uran
          </Typography>
          <Typography variant="caption" color="text.secondary">
            IP Camera Testing
          </Typography>
        </Box>
      </Toolbar>
      <List>
        {items.map((item) => (
          <ListItemButton
            key={item.id}
            selected={activeSection === item.id}
            onClick={() => onSectionChange(item.id)}
          >
            <ListItemIcon>{item.icon}</ListItemIcon>
            <ListItemText primary={item.label} />
          </ListItemButton>
        ))}
      </List>
    </Drawer>
  )
}
