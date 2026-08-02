export type Value = boolean | number | string | null | [number, number]
export interface Choice { value: number; label: string }
export interface Control { kind: 'boolean' | 'number' | 'choice' | 'text' | 'point'; integer?: boolean; min?: number | null; max?: number | null; step?: number; choices?: Choice[] }
export interface Setting { id: string; label: string; help: string; default: Value; control: Control; unit?: string | null }
export interface Group { id: string; title: string; settings: Setting[] }
export interface Preset { id: string; label: string; values: Record<string, Value> }
export interface Demonstration { id: string; label: string; presets: string[] }
export interface Article { id: string; title: string; summary?: string | null; image?: string | null; content_path: string; demonstrations: Demonstration[] }
export interface Catalog { groups: Group[]; presets: Preset[]; articles: Article[] }
