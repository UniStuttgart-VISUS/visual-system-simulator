const MAX_EDGE = 1920
function dimensions(width: number, height: number) {
  const scale = Math.min(1, MAX_EDGE / Math.max(width, height))
  return [Math.max(1, Math.round(width * scale)), Math.max(1, Math.round(height * scale))] as const
}
function rgba(source: CanvasImageSource, width: number, height: number) {
  const [w, h] = dimensions(width, height)
  const canvas = document.createElement('canvas'); canvas.width = w; canvas.height = h
  const context = canvas.getContext('2d', { willReadFrequently: true })!
  context.drawImage(source, 0, 0, w, h)
  return { pixels: context.getImageData(0, 0, w, h).data, width: w, height: h }
}
export interface MediaSource { dispose(): void }
export function flipRows(pixels: Uint8Array, width: number, height: number) {
  const rowBytes = width * 4
  const row = new Uint8Array(rowBytes)
  for (let top = 0, bottom = height - 1; top < bottom; top++, bottom--) {
    const topOffset = top * rowBytes
    const bottomOffset = bottom * rowBytes
    row.set(pixels.subarray(topOffset, topOffset + rowBytes))
    pixels.copyWithin(topOffset, bottomOffset, bottomOffset + rowBytes)
    pixels.set(row, bottomOffset)
  }
}
export async function loadImage(url: string, submit: (pixels: Uint8Array, width: number, height: number) => void): Promise<MediaSource> {
  const image = new Image(); image.decoding = 'async'; image.src = url; await image.decode()
  const frame = rgba(image, image.naturalWidth, image.naturalHeight)
  submit(new Uint8Array(frame.pixels), frame.width, frame.height)
  return { dispose() { image.src = '' } }
}
export async function loadVideo(url: string, submit: (pixels: Uint8Array, width: number, height: number) => void): Promise<MediaSource> {
  const video = document.createElement('video'); video.src = url; video.muted = true; video.loop = true; video.playsInline = true
  await new Promise<void>((resolve, reject) => { video.onloadeddata = () => resolve(); video.onerror = () => reject(video.error) })
  let stopped = false; let callback = 0; const videoFrames = 'requestVideoFrameCallback' in video
  const frame = () => { if (stopped) return; const data = rgba(video, video.videoWidth, video.videoHeight); submit(new Uint8Array(data.pixels), data.width, data.height); callback = videoFrames ? video.requestVideoFrameCallback(frame) : requestAnimationFrame(frame) }
  try { await video.play(); callback = videoFrames ? video.requestVideoFrameCallback(frame) : requestAnimationFrame(frame) } catch (error) { video.removeAttribute('src'); video.load(); throw error }
  return { dispose() { stopped = true; if (videoFrames) video.cancelVideoFrameCallback(callback); else cancelAnimationFrame(callback); video.pause(); video.removeAttribute('src'); video.load() } }
}
