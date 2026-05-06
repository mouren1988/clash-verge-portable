import fs from 'fs'
import fsp from 'fs/promises'
import { createRequire } from 'module'
import path from 'path'

import AdmZip from 'adm-zip'

const target = process.argv.slice(2)[0]
const ARCH_MAP = {
  'x86_64-pc-windows-msvc': 'x64',
  'aarch64-pc-windows-msvc': 'arm64',
}

const PROCESS_MAP = {
  x64: 'x64',
  arm64: 'arm64',
}
const arch = target ? ARCH_MAP[target] : PROCESS_MAP[process.arch]
/// Script for ci
/// 打包绿色版/便携版 (only Windows)
async function resolvePortable() {
  if (process.platform !== 'win32') return

  const candidateReleaseDirs = target
    ? [`./src-tauri/target/${target}/release`, `./target/${target}/release`]
    : ['./src-tauri/target/release', './target/release']
  const releaseDir =
    candidateReleaseDirs.find((dir) => fs.existsSync(dir)) ||
    candidateReleaseDirs[0]
  /// 与 Rust `dirs::init_portable_flag` 一致：exe 旁存在 `Data/` 即便携模式（对齐 iGame-for-Windows）。
  const dataDir = path.join(releaseDir, 'Data')

  if (!fs.existsSync(releaseDir)) {
    throw new Error('could not found the release dir')
  }

  await fsp.mkdir(dataDir, { recursive: true })
  const zip = new AdmZip()

  zip.addLocalFile(path.join(releaseDir, 'clash-verge.exe'))
  zip.addLocalFile(path.join(releaseDir, 'verge-mihomo.exe'))
  zip.addLocalFile(path.join(releaseDir, 'verge-mihomo-alpha.exe'))
  zip.addLocalFolder(path.join(releaseDir, 'resources'), 'resources')
  zip.addLocalFolder(dataDir, 'Data')

  const require = createRequire(import.meta.url)
  const packageJson = require('../package.json')
  const { version } = packageJson
  const zipFile = `Clash.Verge_${version}_${arch}_portable.zip`
  zip.writeZip(zipFile)
  console.log('[INFO]: create portable zip successfully')
}

resolvePortable().catch(console.error)
