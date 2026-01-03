import { useParams } from 'react-router-dom'
import { FolderKanban } from 'lucide-react'

export function ProjectDetailPage() {
  const { id } = useParams<{ id: string }>()

  return (
    <div className="space-y-6">
      <div className="flex items-center gap-4">
        <div className="flex h-16 w-16 items-center justify-center rounded-lg bg-muted">
          <FolderKanban className="h-8 w-8" />
        </div>
        <div>
          <h1 className="text-3xl font-bold">Project</h1>
          <p className="text-muted-foreground">ID: {id}</p>
        </div>
      </div>

      {/* TODO: Implement project detail view */}
      <p className="text-muted-foreground">Project detail view coming soon...</p>
    </div>
  )
}
