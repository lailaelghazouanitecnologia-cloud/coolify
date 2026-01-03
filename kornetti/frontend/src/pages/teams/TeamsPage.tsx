import { Users } from 'lucide-react'
import { Card, CardHeader, CardTitle, CardContent, CardDescription } from '@/components/ui/Card'
import { Badge } from '@/components/ui/Badge'
import { useAuthStore } from '@/stores/auth'

export function TeamsPage() {
  const { teams, currentTeam, setCurrentTeam } = useAuthStore()

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold">Teams</h1>
        <p className="text-muted-foreground">
          Manage your teams and collaborate with others
        </p>
      </div>

      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
        {teams.map((team) => (
          <Card
            key={team.id}
            className={`cursor-pointer transition-colors hover:bg-accent ${
              currentTeam?.id === team.id ? 'ring-2 ring-primary' : ''
            }`}
            onClick={() => setCurrentTeam(team)}
          >
            <CardHeader>
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-3">
                  <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-muted">
                    <Users className="h-5 w-5" />
                  </div>
                  <div>
                    <CardTitle className="text-lg">{team.name}</CardTitle>
                    {team.personal && (
                      <Badge variant="secondary" className="mt-1">
                        Personal
                      </Badge>
                    )}
                  </div>
                </div>
                {currentTeam?.id === team.id && (
                  <Badge variant="success">Active</Badge>
                )}
              </div>
            </CardHeader>
            {team.description && (
              <CardContent>
                <CardDescription>{team.description}</CardDescription>
              </CardContent>
            )}
          </Card>
        ))}
      </div>
    </div>
  )
}
