import { create } from 'zustand'
import { persist } from 'zustand/middleware'
import type { User, Team } from '@/types'

interface AuthState {
  user: User | null
  token: string | null
  currentTeam: Team | null
  teams: Team[]
  isAuthenticated: boolean

  // Actions
  login: (user: User, token: string, teams: Team[]) => void
  logout: () => void
  setCurrentTeam: (team: Team) => void
  updateUser: (user: Partial<User>) => void
}

export const useAuthStore = create<AuthState>()(
  persist(
    (set) => ({
      user: null,
      token: null,
      currentTeam: null,
      teams: [],
      isAuthenticated: false,

      login: (user, token, teams) => {
        const defaultTeam = teams.find((t) => t.personal) || teams[0]
        set({
          user,
          token,
          teams,
          currentTeam: defaultTeam,
          isAuthenticated: true,
        })
      },

      logout: () => {
        set({
          user: null,
          token: null,
          currentTeam: null,
          teams: [],
          isAuthenticated: false,
        })
      },

      setCurrentTeam: (team) => {
        set({ currentTeam: team })
      },

      updateUser: (userData) => {
        set((state) => ({
          user: state.user ? { ...state.user, ...userData } : null,
        }))
      },
    }),
    {
      name: 'kornetti-auth',
      partialize: (state) => ({
        token: state.token,
        user: state.user,
        currentTeam: state.currentTeam,
        teams: state.teams,
        isAuthenticated: state.isAuthenticated,
      }),
    }
  )
)
