---
description: "Use when the user explicitly asks for reconcile-tctbp <absolute-target-repo-path> so the current repository can inspect another repository, detect whether it is new, missing the agent runtime, or already using the agent runtime, and then reconcile that repository's TCTBP state safely."
name: "reconcile-tctbp"
argument-hint: "Absolute target repository path, plus optional source ref, target state or AUTO, backup mode, and whether to include the hook layer"
agent: "agent"
---

# reconcile-tctbp

Use this prompt inside a repository that already uses TCTBP when you want Copilot to handle an explicit `reconcile-tctbp <absolute-target-repo-path>` request and install, adapt, or refresh the TCTBP workflow and optional agent runtime in a different repository.

## Goal

Apply this repository's TCTBP runtime surface to a target repository safely so that Copilot can choose the correct path for one of three cases:

- a brand new repository with no TCTBP files yet
- an existing repository that has some TCTBP workflow files but no custom agent runtime
- an existing repository that already has the custom agent runtime and needs to be refreshed from the current source repository

Depending on the detected or requested state, the target repository should gain or retain:

- a custom TCTBP agent entry point
- a machine-readable workflow policy
- aligned Markdown workflow guidance
- a single reusable TCTBP application prompt
- optional runtime hook enforcement for risky git commands

The current repository is the source of generic workflow logic.
The target repository is the source of repo-specific commands, paths, deployment details, and intentional local deviations.

## Required Inputs

```text
Source TCTBP repository path: <ABSOLUTE_CURRENT_REPOSITORY_PATH_OR_OTHER_SOURCE_REPO>
Target repository path: <ABSOLUTE_TARGET_REPO_PATH>
Target repository state: <AUTO_OR_NEW_REPOSITORY_OR_EXISTING_REPOSITORY_WITHOUT_AGENT_OR_EXISTING_REPOSITORY_WITH_AGENT>
Preferred install/update branch in target repo: <BRANCH_NAME_OR_NULL>
Include hook layer: <YES_OR_NO>
Backup mode for existing repo: <NONE_OR_BRANCH_ONLY_OR_BRANCH_AND_FILE_BACKUPS>
Source ref to use from this repository: <CURRENT_BRANCH_TAG_OR_COMMIT>
Any repo-specific settings that must be preserved exactly: <LIST_OR_NONE>
Any intentional local workflow deviations that must not be normalised away: <LIST_OR_NONE>
```

## Source Files To Use From This Repository

Read these files from the current source repository first:

- `.github/agents/TCTBP.agent.md`
- `.github/TCTBP.json`
- `.github/TCTBP Agent.md`
- `.github/TCTBP Cheatsheet.md`
- `.github/copilot-instructions.md`
- `.github/prompts/Install TCTBP Agent Infrastructure Into Another Repository.prompt.md`

If `Include hook layer` is `YES`, also read:

- `.github/hooks/tctbp-safety.json`
- `scripts/tctbp-pretool-hook.js`

## Target Files To Create Or Update

Install or update these files in the target repository:

- `.github/agents/TCTBP.agent.md`
- `.github/TCTBP.json`
- `.github/TCTBP Agent.md`
- `.github/TCTBP Cheatsheet.md`
- `.github/copilot-instructions.md`
- `.github/prompts/Install TCTBP Agent Infrastructure Into Another Repository.prompt.md`

If `Include hook layer` is `YES`, also install or update:

- `.github/hooks/tctbp-safety.json`
- `scripts/tctbp-pretool-hook.js`

## Required Behaviour

1. Read the source TCTBP files from the current repository.
2. Read the current local versions of every managed target file before editing when they exist.
3. Inspect the target repository structure, commands, version files, deployment scripts, and documentation paths before editing.
4. Preserve repo-specific settings while applying the current runtime model.
5. Replace older onboarding or update prompt files with the single consolidated application prompt.
6. Validate the edited files using lightweight diagnostics appropriate to the change type.
7. Do not perform SHIP, publish, deploy, or handover in the target repo unless explicitly requested.