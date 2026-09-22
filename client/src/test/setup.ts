import '@testing-library/jest-dom/vitest'
import { cleanup } from '@testing-library/svelte'
import { afterEach } from 'vitest'

// jsdom's blob-URL support is unreliable across versions: 30.0.1 returns a
// `blob:nodedata:<uuid>` string, while 30.1.0 throws
// `Cannot read properties of undefined (reading '_buffer')` for a File. Code
// under test (e.g. the avatar preview in ProfileSettingsView) only needs *a*
// string to hand to `img.src`, so stub both sides rather than depend on which
// jsdom happens to be installed.
let objectUrlSeq = 0
const objectUrls = new Set<string>()

URL.createObjectURL = () => {
  const url = `blob:mock/${++objectUrlSeq}`
  objectUrls.add(url)
  return url
}
URL.revokeObjectURL = (url: string) => {
  objectUrls.delete(url)
}

afterEach(() => {
  cleanup()
  objectUrls.clear()
})
