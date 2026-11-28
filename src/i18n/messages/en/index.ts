// English: the source of every message. Each namespace is its own file, so
// work on one part of the window doesn't touch the others.
import common from "./common"
import shell from "./shell"
import dialogs from "./dialogs"
import legal from "./legal"
import library from "./library"
import settings from "./settings"
import format from "./format"

export const en = { common, shell, dialogs, legal, library, settings, format } as const

export type Messages = typeof en
