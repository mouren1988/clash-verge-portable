/**
 * 订阅导入 / 更新后，Mihomo 会重载配置并触发各组、各 provider 的内建测速；
 * 若此时首页「自动延迟检测」再打 checkDelay，会与内核叠峰。在窗口期内跳过首页自动测速。
 */
const DEFAULT_MS = 90_000

let suppressAutoHomeLatencyUntil = 0

export function markSubscriptionKernelReloadDampen(ms: number = DEFAULT_MS) {
  suppressAutoHomeLatencyUntil = Date.now() + Math.max(5_000, ms)
}

export function isSubscriptionKernelReloadDampenActive(): boolean {
  return Date.now() < suppressAutoHomeLatencyUntil
}
