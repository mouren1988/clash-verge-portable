import { lazy } from 'react'

export const Layout = lazy(() => import('./_layout'))
export const ConnectionsPage = lazy(() => import('./connections'))
export const HomePage = lazy(() => import('./home'))
export const ProfilesPage = lazy(() => import('./profiles'))
export const ProxiesPage = lazy(() => import('./proxies'))
export const RulesPage = lazy(() => import('./rules'))
export const SettingsPage = lazy(() => import('./settings'))
export const UnlockPage = lazy(() => import('./unlock'))
