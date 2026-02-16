import Grid from '@mui/material/Grid'
import Card from '@mui/material/Card'
import CardContent from '@mui/material/CardContent'
import Typography from '@mui/material/Typography'

interface DashboardProps {
  runsCompleted: number
  activeRuns: number
  failRate: number
  engineersActivity: string
}

function MetricCard({ title, value }: { title: string; value: string }) {
  return (
    <Card variant="outlined" sx={{ height: '100%' }}>
      <CardContent>
        <Typography variant="body2" color="text.secondary" gutterBottom>
          {title}
        </Typography>
        <Typography variant="h5" fontWeight={700}>
          {value}
        </Typography>
      </CardContent>
    </Card>
  )
}

export default function Dashboard({ runsCompleted, activeRuns, failRate, engineersActivity }: DashboardProps) {
  return (
    <Grid container spacing={2}>
      <Grid size={{ xs: 12, md: 6, lg: 3 }}>
        <MetricCard title="Runs completed" value={String(runsCompleted)} />
      </Grid>
      <Grid size={{ xs: 12, md: 6, lg: 3 }}>
        <MetricCard title="Active runs" value={String(activeRuns)} />
      </Grid>
      <Grid size={{ xs: 12, md: 6, lg: 3 }}>
        <MetricCard title="Fail rate" value={`${failRate.toFixed(1)}%`} />
      </Grid>
      <Grid size={{ xs: 12, md: 6, lg: 3 }}>
        <MetricCard title="Engineers activity" value={engineersActivity} />
      </Grid>
    </Grid>
  )
}
