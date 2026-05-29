import './assets/styles/index.scss'

async function bootstrap() {
  const { mountMainApp } = await import('./mount/main')
  await mountMainApp()
}

bootstrap().catch((error) => {
  console.error('[main.tsx] bootstrap failed:', error)
})

window.addEventListener('error', (event) => {
  console.error('[main.tsx] Global error:', event.error)
})

window.addEventListener('unhandledrejection', (event) => {
  console.error('[main.tsx] Unhandled promise rejection:', event.reason)
})
