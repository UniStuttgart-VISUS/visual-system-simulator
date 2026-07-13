import { describe, expect, it } from 'vitest'
import { applyWebRetina } from './retinaFrame'

describe('applyWebRetina', () => {
  it('preserves the source and returns a copy without active effects', () => {
    const source = new Uint8Array([20, 40, 60, 255])
    const result = applyWebRetina(source, 1, 1, {})
    expect([...result]).toEqual([...source])
    expect(result).not.toBe(source)
  })

  it('applies achromatopsia without changing alpha', () => {
    const result = applyWebRetina(
      new Uint8Array([255, 0, 0, 200]), 1, 1,
      { 'achromatopsia.enabled': true, 'achromatopsia.intensity': 100 },
    )
    expect(result[0]).toBe(result[1])
    expect(result[1]).toBe(result[2])
    expect(result[3]).toBe(200)
  })

  it('darkens the periphery for glaucoma', () => {
    const source = new Uint8Array(3 * 3 * 4).fill(255)
    const result = applyWebRetina(source, 3, 3, {
      'glaucoma.enabled': true,
      'glaucoma.field': 60,
    })
    expect(result[0]).toBeLessThan(result[(4 * 4)])
  })

  it('replaces the selected weak color channel', () => {
    const result = applyWebRetina(
      new Uint8Array([240, 20, 100, 255]), 1, 1,
      {
        'retina.color-deficiency-enabled': true,
        'retina.color-deficiency-type': 0,
        'retina.color-deficiency-intensity': 100,
      },
    )
    expect(result[0]).toBe(60)
    expect(result[1]).toBe(20)
    expect(result[2]).toBe(100)
  })
})
