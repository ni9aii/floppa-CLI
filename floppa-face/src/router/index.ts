import { createRouter, createWebHistory } from 'vue-router'
import { createAppRoutes, installAuthGuard } from 'floppa-web-shared/router'

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: createAppRoutes(),
})

// Logged-out visitors land on the public /welcome page (landing with plans + downloads).
installAuthGuard(router, { unauthenticatedRedirect: 'welcome' })

export default router
