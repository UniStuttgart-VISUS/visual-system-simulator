import type { Value } from './types'

const number = (values: Record<string, Value>, id: string, fallback = 0) =>
  Number(values[id] ?? fallback)

export function applyWebRetina(
  source: Uint8Array,
  width: number,
  height: number,
  values: Record<string, Value>,
) {
  const glaucoma = values['glaucoma.enabled'] === true
  const glaucomaField = number(values, 'glaucoma.field') / 100
  const achromatopsia = values['achromatopsia.enabled'] === true
    ? number(values, 'achromatopsia.intensity', 100) / 100
    : 0
  const nyctalopia = values['nyctalopia.enabled'] === true
    ? number(values, 'nyctalopia.intensity', 100) / 100
    : 0
  const colorDeficiency = values['retina.color-deficiency-enabled'] === true
    ? number(values, 'retina.color-deficiency-intensity', 100) / 100
    : 0
  const colorType = number(values, 'retina.color-deficiency-type')
  const macular = values['macular.enabled'] === true
  const macularStrength = number(
    values,
    values['macular.simple'] === true ? 'macular.simple-intensity' : 'macular.intensity',
    100,
  ) / 100

  const output = source.slice()
  if (!glaucoma && !achromatopsia && !nyctalopia && !colorDeficiency && !macular) return output

  for (let offset = 0; offset < output.length; offset += 4) {
    const pixel = offset / 4
    const x = (pixel % width) / width * 2 - 1
    const y = Math.floor(pixel / width) / height * 2 - 1
    const radius = Math.hypot(x, y)
    let spatial = 1
    if (glaucoma) spatial *= Math.max(.04, 1 - Math.max(0, radius - (1 - glaucomaField)) * 5)
    if (macular) spatial *= 1 - (1 - Math.min(1, radius * 3)) * macularStrength * .92

    let r = output[offset]
    let g = output[offset + 1]
    let b = output[offset + 2]
    const gray = r * .299 + g * .587 + b * .114
    r += (gray - r) * achromatopsia
    g += (gray - g) * achromatopsia
    b += (gray - b) * achromatopsia

    if (colorDeficiency) {
      const weak = colorType === 1 ? g : colorType === 2 ? b : r
      const replacement = colorType === 1 ? (r + b) / 2 : colorType === 2 ? (r + g) / 2 : (g + b) / 2
      const corrected = weak + (replacement - weak) * colorDeficiency
      if (colorType === 1) g = corrected
      else if (colorType === 2) b = corrected
      else r = corrected
    }

    const luminance = Math.min(1, gray / 51)
    const nightFactor = 1 - nyctalopia * .75 * (1 - luminance)
    output[offset] = r * spatial * nightFactor
    output[offset + 1] = g * spatial * nightFactor
    output[offset + 2] = b * spatial * nightFactor
  }
  return output
}
