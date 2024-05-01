import { parse } from '@next/vendored/stacktrace-parser'
import type { StackFrame } from '@next/vendored/stacktrace-parser'

const regexNextStatic = /\/_next(\/static\/.+)/

export function parseStack(stack: string): StackFrame[] {
  const frames = parse(stack)
  return frames.map((frame) => {
    try {
      const url = new URL(frame.file!)
      const res = regexNextStatic.exec(url.pathname)
      if (res) {
        const distDir = process.env.__NEXT_DIST_DIR
          ?.replace(/\\/g, '/')
          ?.replace(/\/$/, '')
        if (distDir) {
          frame.file = 'file://' + distDir.concat(res.pop()!) + url.search
        }
      }
    } catch {}
    return frame
  })
}
