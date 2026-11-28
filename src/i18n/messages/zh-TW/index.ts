import type { Messages } from "../en"
import type { Translation } from "../../types"
import common from "./common"
import shell from "./shell"
import dialogs from "./dialogs"
import legal from "./legal"
import library from "./library"
import settings from "./settings"
import format from "./format"

export const zhTW = { common, shell, dialogs, legal, library, settings, format } satisfies Translation<Messages>
