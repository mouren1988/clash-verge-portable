import { ResizeObserver } from '@juggle/resize-observer'
import { QueryClientProvider } from '@tanstack/react-query'
import { ComposeContextProvider } from 'foxact/compose-context-provider'
import React, { Suspense } from 'react'
import { createRoot } from 'react-dom/client'
import { RouterProvider } from 'react-router'
import { MihomoWebSocket } from 'tauri-plugin-mihomo-api'

import { BaseErrorBoundary } from '@/components/base'
import { router } from '@/pages/_routers'
import { AppDataProvider } from '@/providers/app-data-provider'
import { WindowProvider } from '@/providers/window'
import { FALLBACK_LANGUAGE, initializeLanguage } from '@/services/i18n'
import {
  preloadAppData,
  resolveThemeMode,
  getPreloadConfig,
} from '@/services/preload'
import { queryClient } from '@/services/query-client'
import {
  LoadingCacheProvider,
  ThemeModeProvider,
  UpdateStateProvider,
} from '@/services/states'
import { disableWebViewShortcuts } from '@/utils/disable-webview-shortcuts'

export async function mountMainApp() {
  if (!window.ResizeObserver) {
    window.ResizeObserver = ResizeObserver
  }

  const container = document.getElementById('root')
  if (!container) {
    throw new Error(`No container 'root' found to render application`)
  }

  disableWebViewShortcuts()

  const renderApp = (initialThemeMode: 'light' | 'dark') => {
    const contexts = [
      <ThemeModeProvider key="theme" initialState={initialThemeMode} />,
      <LoadingCacheProvider key="loading" />,
      <UpdateStateProvider key="update" />,
    ]

    createRoot(container).render(
      <React.StrictMode>
        <ComposeContextProvider contexts={contexts}>
          <BaseErrorBoundary>
            <QueryClientProvider client={queryClient}>
              <WindowProvider>
                <AppDataProvider>
                  <Suspense fallback={null}>
                    <RouterProvider router={router} />
                  </Suspense>
                </AppDataProvider>
              </WindowProvider>
            </QueryClientProvider>
          </BaseErrorBoundary>
        </ComposeContextProvider>
      </React.StrictMode>,
    )
  }

  try {
    const { initialThemeMode } = await preloadAppData()
    renderApp(initialThemeMode)
  } catch (error) {
    console.error('[main] preload failed, using fallback language:', error)
    try {
      await initializeLanguage(FALLBACK_LANGUAGE)
    } catch (fallbackError) {
      console.error('[main] language fallback failed:', fallbackError)
    }
    renderApp(resolveThemeMode(getPreloadConfig()))
  }

  window.addEventListener('beforeunload', () => {
    MihomoWebSocket.cleanupAll()
    queryClient.clear()
  })

  window.addEventListener('DOMContentLoaded', () => {
    MihomoWebSocket.cleanupAll()
  })
}
