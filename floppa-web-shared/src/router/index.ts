import type { RouteRecordRaw, Router } from 'vue-router'
import { useAuthStore } from '../stores'

export function createAppRoutes(): RouteRecordRaw[] {
  return [
    {
      path: '/login',
      name: 'login',
      component: () => import('../views/LoginView.vue'),
    },
    {
      // Public landing for logged-out visitors: description, plans, downloads, login CTA.
      path: '/welcome',
      name: 'welcome',
      component: () => import('../views/InfoView.vue'),
      props: { variant: 'landing' },
    },
    {
      path: '/',
      name: 'dashboard',
      component: () => import('../views/user/DashboardView.vue'),
      meta: { requiresAuth: true },
    },
    {
      // In-app Info tab for authenticated users (same content, no login CTA).
      path: '/info',
      name: 'info',
      component: () => import('../views/InfoView.vue'),
      props: { variant: 'tab' },
      meta: { requiresAuth: true },
    },
    {
      path: '/peers',
      name: 'peers',
      component: () => import('../views/user/PeersView.vue'),
      meta: { requiresAuth: true },
    },
    {
      path: '/account',
      name: 'account',
      component: () => import('../views/user/AccountView.vue'),
      meta: { requiresAuth: true },
    },
    {
      path: '/admin',
      name: 'admin-dashboard',
      component: () => import('../views/admin/DashboardView.vue'),
      meta: { requiresAuth: true, requiresAdmin: true },
    },
    {
      path: '/admin/peers',
      name: 'admin-peers',
      component: () => import('../views/admin/PeersView.vue'),
      meta: { requiresAuth: true, requiresAdmin: true },
    },
    {
      path: '/admin/users',
      name: 'admin-users',
      component: () => import('../views/admin/UsersView.vue'),
      meta: { requiresAuth: true, requiresAdmin: true },
    },
    {
      path: '/admin/users/:id',
      name: 'admin-user-detail',
      component: () => import('../views/admin/UserDetailView.vue'),
      meta: { requiresAuth: true, requiresAdmin: true },
    },
    {
      path: '/admin/plans',
      name: 'admin-plans',
      component: () => import('../views/admin/PlansView.vue'),
      meta: { requiresAuth: true, requiresAdmin: true },
    },
    {
      path: '/admin/installations',
      name: 'admin-installations',
      component: () => import('../views/admin/InstallationsView.vue'),
      meta: { requiresAuth: true, requiresAdmin: true },
    },
    {
      path: '/admin/vless',
      name: 'admin-vless',
      component: () => import('../views/admin/VlessPeersView.vue'),
      meta: { requiresAuth: true, requiresAdmin: true },
    },
  ]
}

/** Raw Mini App initData, or null when not running inside a Telegram Mini App. */
function getTelegramInitData(): string | null {
  try {
    const initData = (window as { Telegram?: { WebApp?: { initData?: string } } }).Telegram?.WebApp
      ?.initData
    return initData && initData.length > 0 ? initData : null
  } catch {
    return null
  }
}

/** Parse Telegram user ID from Mini App initData (URL-encoded). */
function getTelegramUserIdFromInitData(): number | null {
  try {
    const initData = getTelegramInitData()
    if (!initData) return null
    const userJson = new URLSearchParams(initData).get('user')
    if (!userJson) return null
    const id = JSON.parse(userJson).id
    return typeof id === 'number' ? id : null
  } catch {
    return null
  }
}

export interface AuthGuardOptions {
  /**
   * Route name to send logged-out visitors to when they hit a protected route.
   * The web uses 'welcome' (the public landing); the Tauri client uses 'login'.
   */
  unauthenticatedRedirect?: string
}

export function installAuthGuard(router: Router, options: AuthGuardOptions = {}): void {
  const unauthenticatedRedirect = options.unauthenticatedRedirect ?? 'login'

  router.beforeEach((to) => {
    const auth = useAuthStore()

    // If in Mini App and a different Telegram account opened the app, force re-login
    const tgUserId = getTelegramUserIdFromInitData()
    if (tgUserId !== null && auth.isAuthenticated && auth.telegramId !== tgUserId) {
      auth.logout()
    }

    if (to.meta.requiresAuth && !auth.isAuthenticated) {
      // Inside a Telegram Mini App, always go through /login: LoginView auto-logs in
      // via initData and forwards to the dashboard, so the user never sees the public
      // landing. Outside Telegram, logged-out visitors land on the configured page.
      if (getTelegramInitData() !== null) {
        return { name: 'login' }
      }
      return { name: unauthenticatedRedirect }
    }

    if (to.meta.requiresAdmin && !auth.isAdmin) {
      return { name: 'dashboard' }
    }

    if (to.name === 'login' && auth.isAuthenticated) {
      return { name: 'dashboard' }
    }
  })
}
