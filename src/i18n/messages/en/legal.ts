// The legal notice and the responsibility warnings (notice-dialog.tsx, src/features/shared/responsibility.tsx).
export default {
  /**
   * LEGAL.md's notice, word for word: keep the two in step (LEGAL.md is also in the README and
   * the installer). The English notice governs; translations say so in `translationNote`.
   * The bold lead of each point is its first sentence.
   */
  notice: {
    title: "Before you start",
    intro: "VILauncher is a project manager. It keeps community projects in one place and downloads, builds and starts them, so you don't have to do it by hand. That is all it does.",
    points: {
      game: "<b>You must provide the game.</b> VILauncher does not include, download, host or distribute any game, game build, game executable, game files or other copyrighted game content. It works only with a copy of a game that you own and supply yourself, from a folder on your own PC. Without one, it cannot run any project that needs that game.",
      responsibility: "<b>You are responsible for what happens.</b> The projects it manages are third-party software made by other people. VILauncher fetches their source code from where their authors publish it, builds it, and runs it on your PC at your request. It does not review, certify or guarantee any project. You alone decide what you add, install, build and run, and you are solely responsible for doing so, and for any consequences, including to your PC, your data, your accounts and other people's rights. That includes having the right to use the game files you supply, and following the laws, licences and terms that apply to you.",
      code: "<b>Running code.</b> Building a project runs its build scripts and compiles its code. Running a project runs its program. Both can read and change files you can access, use the network, and use your hardware. VILauncher shows these warnings before the first repo or ZIP import, before an update, and before an AI agent may act. Accepting a warning is your decision and your responsibility.",
      warranty: "<b>No warranty.</b> VILauncher is free software, provided \"as is\", without warranty of any kind, express or implied, as set out in sections 15 and 16 of the GNU General Public License version 3. Its authors and contributors are not liable for any damage or loss arising from its use, or from the use of any project it manages.",
      affiliation: "<b>Not affiliated.</b> VILauncher and the VIPortLive team are not affiliated with, endorsed by or connected to Rockstar Games, Take-Two Interactive, or any other game publisher or developer, including the publisher of the game these community projects target. Game titles, names and marks belong to their owners. Project names and descriptions belong to their authors.",
      licence: "<b>Licence and forks.</b> VILauncher is licensed under the GNU General Public License version 3 or later (see LICENSE). You may fork, change and redistribute it under that licence. Any version you distribute must remain under the GPL, include its source code, keep the copyright and licence notices, credit the authors and say what you changed. It is a fork of Mandyhub, and the Mandyhub copyright and credits are kept.",
      takedowns: "<b>Takedowns.</b> When a project is taken down, VIPortLive marks it, and VILauncher hides it by default. The player can show it, but it then opens VIPortLive's page and cannot be installed from the launcher. VILauncher does not host any project's code, so takedown requests should be sent to the host where the code lives, and to VIPortLive for the list.",
      data: "<b>Your data.</b> Settings, saves and projects are stored on your PC in the folders listed in the README. VILauncher sends no personal data anywhere. Its problem-report button builds text on your PC and opens a GitHub issue page for you; you choose whether to submit it, and you can read it first. Names and your folders are removed from reports before you see them.",
    },
    closing: "By using VILauncher you confirm that you have read and agree to this notice.",
    /** Shown under a translated notice only (not in English). */
    translationNote: "This translation is for convenience only. If it differs from the English notice, the English notice applies.",
    agree: "I understand and agree",
    quit: "Quit",
  },
  /** The warning before each risky step (ResponsibilityId). Keep every risk listed. */
  responsibility: {
    risks: "What could happen",
    accept: "I understand, and I take responsibility for what this does on my PC.",
    importRepo: {
      title: "Before you import this project",
      body: "Importing a repo downloads its code and runs it on this PC. Its build scripts and the programs it builds can read and change files you can access, use the network, and use your graphics card. VILauncher checks the repo's game files and verified project list but cannot make unknown code safe. Only import repos you trust, and read the code if you can.",
      risks: {
        build: "It runs build scripts and compiles the code.",
        download: "It may download more code and tools.",
        write: "It can write to folders your user account can write to.",
        saves: "The saves VILauncher keeps are separate and an import does not touch them, but the project's own code might.",
      },
    },
    agentAccess: {
      title: "Let AI agents help with builds?",
      body: "Turning this on lets AI agents (for example Claude Code or opencode) read this launcher's logs and projects, and, when you confirm each step, apply a patch to a project's source and run its build. An agent can make mistakes, and its patch runs on this PC.",
      risks: {
        read: "Agents read logs and folder names. Your user name is removed from what is shown.",
        scope: "Patches only touch a project's own source folder.",
        confirm: "Every change needs your confirmation. Nothing is committed or pushed.",
        off: "You can turn this off at any time.",
      },
    },
    agentPatch: {
      title: "Apply this patch?",
      body: "This patch will change the project's source and run its build on this PC. Saves are not touched. The patch is written by an agent and has not been reviewed by the project's authors.",
    },
  },
} as const
