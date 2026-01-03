import { useState } from 'react'
import { Link } from 'react-router-dom'
import { Plus, Server, MoreVertical, Trash2, RefreshCw } from 'lucide-react'
import { toast } from 'sonner'
import { Button } from '@/components/ui/Button'
import { Card, CardHeader, CardTitle, CardContent } from '@/components/ui/Card'
import { Badge } from '@/components/ui/Badge'
import { useServers, useDeleteServer, useValidateServer } from '@/api/servers'

export function ServersPage() {
  const { data: servers, isLoading } = useServers()
  const deleteServer = useDeleteServer()
  const validateServer = useValidateServer()

  const handleDelete = async (id: string, name: string) => {
    if (!confirm(`Are you sure you want to delete "${name}"?`)) return

    try {
      await deleteServer.mutateAsync(id)
      toast.success(`Server "${name}" deleted`)
    } catch (error) {
      toast.error('Failed to delete server')
    }
  }

  const handleValidate = async (id: string) => {
    try {
      await validateServer.mutateAsync(id)
      toast.success('Server validation started')
    } catch (error) {
      toast.error('Failed to validate server')
    }
  }

  const getStatusBadge = (status: string) => {
    switch (status) {
      case 'ready':
        return <Badge variant="success">Ready</Badge>
      case 'reachable':
        return <Badge variant="success">Reachable</Badge>
      case 'validating':
      case 'installing':
        return <Badge variant="warning">In Progress</Badge>
      case 'unreachable':
      case 'error':
        return <Badge variant="error">{status}</Badge>
      default:
        return <Badge variant="secondary">{status}</Badge>
    }
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">Servers</h1>
          <p className="text-muted-foreground">
            Manage your infrastructure and server connections
          </p>
        </div>
        <Button asChild>
          <Link to="/servers/new">
            <Plus className="h-4 w-4" />
            Add Server
          </Link>
        </Button>
      </div>

      {isLoading ? (
        <div className="flex items-center justify-center py-12">
          <div className="h-8 w-8 animate-spin rounded-full border-4 border-primary border-t-transparent" />
        </div>
      ) : servers && servers.length > 0 ? (
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          {servers.map((server) => (
            <Card key={server.id} className="group relative">
              <CardHeader className="pb-2">
                <div className="flex items-start justify-between">
                  <div className="flex items-center gap-3">
                    <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-muted">
                      <Server className="h-5 w-5" />
                    </div>
                    <div>
                      <CardTitle className="text-lg">{server.name}</CardTitle>
                      <p className="text-sm text-muted-foreground">
                        {server.ip}:{server.port}
                      </p>
                    </div>
                  </div>
                  {getStatusBadge(server.status)}
                </div>
              </CardHeader>
              <CardContent>
                <div className="space-y-2 text-sm">
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">User</span>
                    <span>{server.user}</span>
                  </div>
                  {server.provider && (
                    <div className="flex justify-between">
                      <span className="text-muted-foreground">Provider</span>
                      <span className="capitalize">{server.provider}</span>
                    </div>
                  )}
                  {server.region && (
                    <div className="flex justify-between">
                      <span className="text-muted-foreground">Region</span>
                      <span>{server.region}</span>
                    </div>
                  )}
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">Docker</span>
                    <span>
                      {server.settings.docker_installed
                        ? server.settings.docker_version || 'Installed'
                        : 'Not installed'}
                    </span>
                  </div>
                </div>

                <div className="mt-4 flex gap-2">
                  <Button variant="outline" size="sm" className="flex-1" asChild>
                    <Link to={`/servers/${server.id}`}>View Details</Link>
                  </Button>
                  <Button
                    variant="outline"
                    size="icon"
                    onClick={() => handleValidate(server.id)}
                    disabled={validateServer.isPending}
                  >
                    <RefreshCw className="h-4 w-4" />
                  </Button>
                  <Button
                    variant="outline"
                    size="icon"
                    onClick={() => handleDelete(server.id, server.name)}
                    disabled={deleteServer.isPending}
                  >
                    <Trash2 className="h-4 w-4 text-destructive" />
                  </Button>
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      ) : (
        <Card>
          <CardContent className="flex flex-col items-center justify-center py-12">
            <Server className="h-12 w-12 text-muted-foreground" />
            <h3 className="mt-4 text-lg font-medium">No servers yet</h3>
            <p className="mt-2 text-center text-sm text-muted-foreground">
              Get started by adding your first server. You can connect to any
              Linux server with SSH access.
            </p>
            <Button asChild className="mt-6">
              <Link to="/servers/new">
                <Plus className="h-4 w-4" />
                Add Server
              </Link>
            </Button>
          </CardContent>
        </Card>
      )}
    </div>
  )
}
