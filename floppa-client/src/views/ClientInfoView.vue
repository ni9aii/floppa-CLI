<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { openUrl } from '@tauri-apps/plugin-opener'
import { InfoView } from 'floppa-web-shared/views'

const { t } = useI18n()
const appVersion = __APP_VERSION__

// On the client, external links (GitHub, Telegram, downloads) must open in the system browser.
function openExternal(url: string) {
  void openUrl(url)
}
</script>

<template>
  <InfoView :open-external="openExternal">
    <template #app-info>
      <span class="text-sm text-[var(--ui-text-muted)]">
        {{ t('info.version', { version: appVersion }) }}
      </span>
    </template>
  </InfoView>
</template>
