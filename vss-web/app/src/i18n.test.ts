import { messages } from './i18n'
test('shell messages have matching locales', () => { expect(Object.keys(messages.de.app).sort()).toEqual(Object.keys(messages.en.app).sort()) })
