import { Server, FolderKanban, Rocket, AlertCircle } from 'lucide-react'
import { Link } from 'react-router-dom'
import { Card, CardHeader, CardTitle, CardContent } from '@/components/ui/Card'
import { Badge } from '@/components/ui/Badge'
import { useServers } from '@/api/servers'

export function DashboardPage() {
  const { data: servers, isLoading } = useServers()

  const stats = {
    servers: servers?.length || 0,
    serversOnline: servers?.filter((s) => s.status === 'ready').length || 0,
    projects: 0, // TODO: fetch from API
    deployments: 0, // TODO: fetch from API
  }

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold">Dashboard</h1>
        <p className="text-muted-foreground">
          Overview of your infrastructure and deployments
        </p>
      </div>

      {/* Stats Grid */}
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium">Servers</CardTitle>
            <Server className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{stats.servers}</div>
            <p className="text-xs text-muted-foreground">
              {stats.serversOnline} online
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium">Projects</CardTitle>
            <FolderKanban className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{stats.projects}</div>
            <p className="text-xs text-muted-foreground">
              Across all environments
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium">Deployments</CardTitle>
            <Rocket className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{stats.deployments}</div>
            <p className="text-xs text-muted-foreground">This week</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between pb-2">
            <CardTitle className="text-sm font-medium">Alerts</CardTitle>
            <AlertCircle className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">0</div>
            <p className="text-xs text-muted-foreground">Active issues</p>
          </CardContent>
        </Card>
      </div>

      {/* Recent Servers */}
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <CardTitle>Servers</CardTitle>
            <Link
              to="/servers"
              className="text-sm text-primary hover:underline"
            >
              View all
            </Link>
          </div>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="flex items-center justify-center py-8">
              <div className="h-8 w-8 animate-spin rounded-full border-4 border-primary border-t-transparent" />
            </div>
          ) : servers && servers.length > 0 ? (
            <div className="space-y-4">
              {servers.slice(0, 5).map((server) => (
                <Link
                  key={server.id}
                  to={`/servers/${server.id}`}
                  className="flex items-center justify-between rounded-lg border p-4 transition-colors hover:bg-accent"
                >
                  <div className="flex items-center gap-4">
                    <Server className="h-8 w-8 text-muted-foreground" />
                    <div>
                      <p className="font-medium">{server.name}</p>
                      <p className="text-sm text-muted-foreground">
                        {server.ip}:{server.port}
                      </p>
                    </div>
                  </div>
                  <Badge
                    variant={
                      server.status === 'ready'
                        ? 'success'
                        : server.status === 'error'
                        ? 'error'
                        : 'warning'
                    }
                  >
                    {server.status}
                  </Badge>
                </Link>
              ))}
            </div>
          ) : (
            <div className="flex flex-col items-center justify-center py-8 text-center">
              <Server className="h-12 w-12 text-muted-foreground" />
              <h3 className="mt-4 text-lg font-medium">No servers yet</h3>
              <p className="mt-2 text-sm text-muted-foreground">
                Add your first server to get started
              </p>
              <Link to="/servers">
                <Badge className="mt-4" variant="default">
                  Add Server
                </Badge>
              </Link>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  )
}
