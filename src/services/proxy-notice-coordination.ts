/** 与「冷启动时配置里已开启 TUN/系统代理」区分，避免与 useProxyBootNotice 叠发两条成功提示。 */
let lastUserDroveProxyTogglesAt = 0

export function markUserDroveProxyToggles() {
  lastUserDroveProxyTogglesAt = Date.now()
}

export function isRecentUserDroveProxyToggles(withinMs: number) {
  if (lastUserDroveProxyTogglesAt <= 0) {
    return false
  }
  return Date.now() - lastUserDroveProxyTogglesAt < withinMs
}
