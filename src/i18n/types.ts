// The shape of the window's messages. English (messages/en) is the source:
// every other language is a `Translation` of it, so a missing or extra key is
// a type error. See docs/I18N.md.

/**
 * A message that depends on a number: `{ one: "{count} project", other: "{count} projects" }`.
 * The form is picked with `Intl.PluralRules` for the `count` parameter. `other` is always
 * needed; languages without plurals (Chinese) give only `other`.
 */
export interface PluralForms {
  zero?: string
  one?: string
  two?: string
  few?: string
  many?: string
  other: string
}

/** A namespace of messages: strings, plural forms, or nested groups. */
export interface MessageTree {
  readonly [key: string]: string | PluralForms | MessageTree
}

/** A translation of the English messages `T`: the same keys, each translated. */
export type Translation<T> = {
  [K in keyof T]: T[K] extends string ? string : T[K] extends { other: string } ? PluralForms : Translation<T[K]>
}

/** Every message's dotted key in `T`, such as `"settings.language.label"`. */
export type MessageKey<T> = {
  [K in keyof T & string]: T[K] extends string | { other: string } ? K : `${K}.${MessageKey<T[K]>}`
}[keyof T & string]

/** The message at the dotted key `K` in `T`. */
export type MessageAt<T, K extends string> = K extends `${infer Head}.${infer Rest}`
  ? Head extends keyof T
    ? MessageAt<T[Head], Rest>
    : never
  : K extends keyof T
    ? T[K]
    : never

/** The `{name}` placeholders in a message (`{{` is a brace, not a placeholder). */
type Placeholders<S> = S extends `${string}{{${infer Rest}`
  ? Placeholders<Rest>
  : S extends `${string}{${infer Name}}${infer Rest}`
    ? Name | Placeholders<Rest>
    : never

/** The parameter names a message needs: its placeholders, and `count` for plural forms. */
export type ParamNames<M> = M extends string
  ? Placeholders<M>
  : M extends { other: string }
    ? "count" | Placeholders<M[keyof M & ("zero" | "one" | "two" | "few" | "many" | "other")]>
    : never

/** A value put into a message. Numbers are written the language's way. */
export type ParamValue = string | number
