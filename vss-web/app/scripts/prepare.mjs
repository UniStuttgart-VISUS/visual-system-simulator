import { cp, mkdir, rm } from 'node:fs/promises'
import { spawnSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import path from 'node:path'

const app = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const root = path.resolve(app, '..', '..')
const wasm = spawnSync('wasm-pack', ['build', path.join(root, 'vss-web'), '--target', 'web', '--out-dir', path.join(app, 'src', 'wasm')], { stdio: 'inherit' })
if (wasm.status !== 0) process.exit(wasm.status ?? 1)
const publicDir = path.join(app, 'public')
await rm(publicDir, { recursive: true, force: true })
await mkdir(path.join(publicDir, 'assets'), { recursive: true })
await cp(path.join(root, 'assets', 'marketplace.rgbd.png'), path.join(publicDir, 'assets', 'marketplace.rgbd.png'))
await cp(path.join(root, 'vss-catalog', 'articles'), path.join(publicDir, 'articles'), { recursive: true, filter: source => !source.endsWith('.md') })
await cp(path.join(app, 'assets', 'icons'), path.join(publicDir, 'icons'), { recursive: true })
await cp(path.join(app, 'assets', 'manifest.webmanifest'), path.join(publicDir, 'manifest.webmanifest'))
