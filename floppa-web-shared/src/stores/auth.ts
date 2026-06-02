import { ref, computed } from 'vue'
import { defineStore } from 'pinia'
import type { AuthUserInfo } from '../client/types.gen'
import { getMyAvatar } from '../client/sdk.gen'

const TOKEN_KEY = 'floppa-token'
const USER_KEY = 'floppa-user'
const AVATAR_KEY = 'floppa-avatar'
const TELEGRAM_ID_KEY = 'floppa-telegram-id'

export const useAuthStore = defineStore('auth', () => {
  const token = ref<string | null>(localStorage.getItem(TOKEN_KEY))
  const user = ref<AuthUserInfo | null>(
    (() => {
      const stored = localStorage.getItem(USER_KEY)
      if (!stored) return null
      try {
        const parsed = JSON.parse(stored)
        if (typeof parsed?.id !== 'number') return null
        return parsed
      } catch {
        return null
      }
    })(),
  )

  const telegramId = ref<number | null>(
    (() => {
      const stored = localStorage.getItem(TELEGRAM_ID_KEY)
      if (!stored) return null
      const n = Number(stored)
      return Number.isFinite(n) ? n : null
    })(),
  )

  const cachedAvatar = ref<string | null>(localStorage.getItem(AVATAR_KEY))
  // Avatars are served from our own origin (server-cached); the raw Telegram photo_url is
  // unreachable from clients in Russia, so it's not used as a fallback.
  const avatarUrl = computed(() => cachedAvatar.value ?? undefined)

  const isAuthenticated = computed(() => !!token.value)
  const isAdmin = computed(() => user.value?.is_admin ?? false)

  /// Fetch the current user's avatar from the server (Bearer-authed) and cache it as a data URL.
  /// The server caches it asynchronously on login, so on a fresh login it may 404 briefly — retry
  /// a couple of times before giving up.
  function cacheAvatar(retries = 3) {
    getMyAvatar({ parseAs: 'blob' })
      .then(({ data, response }) => {
        if (response.status === 404) {
          if (retries > 0) setTimeout(() => cacheAvatar(retries - 1), 4000)
          return
        }
        if (!response.ok || !(data instanceof Blob)) return
        const reader = new FileReader()
        reader.onloadend = () => {
          const dataUrl = reader.result as string
          cachedAvatar.value = dataUrl
          localStorage.setItem(AVATAR_KEY, dataUrl)
        }
        reader.readAsDataURL(data)
      })
      .catch(() => {})
  }

  function setAuth(newToken: string, newUser: AuthUserInfo, newTelegramId?: number) {
    token.value = newToken
    user.value = newUser
    localStorage.setItem(TOKEN_KEY, newToken)
    localStorage.setItem(USER_KEY, JSON.stringify(newUser))
    if (newTelegramId !== undefined) {
      telegramId.value = newTelegramId
      localStorage.setItem(TELEGRAM_ID_KEY, String(newTelegramId))
    }
    cacheAvatar()
  }

  function logout() {
    token.value = null
    user.value = null
    cachedAvatar.value = null
    localStorage.removeItem(TOKEN_KEY)
    localStorage.removeItem(USER_KEY)
    localStorage.removeItem(AVATAR_KEY)
    telegramId.value = null
    localStorage.removeItem(TELEGRAM_ID_KEY)
  }

  // Refresh the avatar from the server on startup when authenticated but not yet cached.
  if (token.value && !cachedAvatar.value) {
    cacheAvatar()
  }

  function getToken(): string | null {
    return token.value
  }

  return {
    token,
    user,
    telegramId,
    isAuthenticated,
    isAdmin,
    avatarUrl,
    setAuth,
    logout,
    getToken,
  }
})
