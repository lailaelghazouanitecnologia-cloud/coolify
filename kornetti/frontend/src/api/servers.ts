import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { get, post, put, del } from './client'
import type { Server, PaginatedResponse } from '@/types'

const SERVERS_KEY = ['servers']

// Queries
export function useServers() {
  return useQuery({
    queryKey: SERVERS_KEY,
    queryFn: () => get<Server[]>('/servers'),
  })
}

export function useServer(id: string) {
  return useQuery({
    queryKey: [...SERVERS_KEY, id],
    queryFn: () => get<Server>(`/servers/${id}`),
    enabled: !!id,
  })
}

export function useServerResources(id: string) {
  return useQuery({
    queryKey: [...SERVERS_KEY, id, 'resources'],
    queryFn: () => get<ServerResources>(`/servers/${id}/resources`),
    enabled: !!id,
    refetchInterval: 30000, // Refresh every 30 seconds
  })
}

// Mutations
export function useCreateServer() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: (data: CreateServerInput) => post<Server>('/servers', data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: SERVERS_KEY })
    },
  })
}

export function useUpdateServer() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: ({ id, data }: { id: string; data: UpdateServerInput }) =>
      put<Server>(`/servers/${id}`, data),
    onSuccess: (_, { id }) => {
      queryClient.invalidateQueries({ queryKey: [...SERVERS_KEY, id] })
      queryClient.invalidateQueries({ queryKey: SERVERS_KEY })
    },
  })
}

export function useDeleteServer() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: (id: string) => del<void>(`/servers/${id}`),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: SERVERS_KEY })
    },
  })
}

export function useValidateServer() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: (id: string) => post<void>(`/servers/${id}/validate`),
    onSuccess: (_, id) => {
      queryClient.invalidateQueries({ queryKey: [...SERVERS_KEY, id] })
    },
  })
}

export function useInstallDocker() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: (id: string) => post<void>(`/servers/${id}/install-docker`),
    onSuccess: (_, id) => {
      queryClient.invalidateQueries({ queryKey: [...SERVERS_KEY, id] })
    },
  })
}

// Types
interface CreateServerInput {
  name: string
  description?: string
  ip: string
  port: number
  user: string
  private_key_id: string
}

interface UpdateServerInput {
  name?: string
  description?: string
  settings?: Partial<Server['settings']>
}

interface ServerResources {
  cpu_usage: number
  memory_used: number
  memory_total: number
  disk_used: number
  disk_total: number
  containers_running: number
  containers_total: number
}
