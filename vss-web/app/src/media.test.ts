import { describe, expect, it } from 'vitest'
import { flipRows } from './media'

describe('flipRows', () => {
  it('moves an RGB-D color half below the depth half', () => {
    const pixels = new Uint8Array([
      1, 1, 1, 255,
      2, 2, 2, 255,
      3, 3, 3, 255,
      4, 4, 4, 255,
    ])

    flipRows(pixels, 1, 4)

    expect([...pixels]).toEqual([
      4, 4, 4, 255,
      3, 3, 3, 255,
      2, 2, 2, 255,
      1, 1, 1, 255,
    ])
  })
})
