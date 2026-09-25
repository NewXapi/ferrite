---
name: web-spec-check
description: 'Web refactor gate operator: inspect a complete web file plus its diff against the Ferrite page-crate specification, run the configured jev checks, and report the three typed decisions (yes/no, multi-select booleans, choice, score) for review. Use when reviewing web-dev refactors, before pushing web changes, or when auditing a web PR against todo/web-spec.md.'
---

# Web spec check

## Purpose

This skill is the operating procedure for the `web_refactor` custom spec. It is for the dev/review loop, not for deciding the product design.

The spec lives at:

```text
.githooks/spec/custom/web_refactor.json
.githooks/spec/quality/checklist_web_refactor.yaml
```

The design/review record lives at:

```text
todo/web-refactor-gate.md
```

The method source is:

```text
todo/web-spec.md
```

## State contract

For every changed web Rust file, build one state containing all of the following. Do not judge from a path name or a short hunk alone:

```json
{
  "path": "crates/web/<page-crate>/<file>.rs",
  "file": "<complete current file text>",
  "diff": "<git diff for this exact file against the review base>",
  "intent": "<the web_refactor rule intent from the custom spec>",
  "context": [
    "todo/web-spec.md",
    ".agent/rules/web-lanes.md",
    ".agent/tasks/dev-web-lane.md"
  ]
}
```

Requirements:

- Use the complete current file, not only changed lines.
- Use the exact file diff from the recorded base SHA to the current SHA.
- Keep the repository-relative path in the state.
- Include the relevant page-crate and shared-ui context. The rule config controls which context files are inlined.
- If a file is deleted, classify the deletion as a structural move only when the destination is visible in the same diff; otherwise mark the evidence missing.
- Do not silently truncate a long file. If the judge state limit is reached, include a visible truncation marker and record the truncation in the report.

## Decision types

The custom spec uses all three jev primitives. Do not collapse them into one generic question.

### 1. Yes/no (`noul` / bool)

Used for each independent refactor obligation:

- `has_refactor_debt`
- `page_shape`
- `style_unification`
- `shared_component_ownership`
- `composition_discipline`
- `state_and_interaction_boundary`
- `i18n_constants`
- `component_documentation`
- `behavior_drift`
- `release_blocker`

These independent yes/no results are the multi-select result. A file can be true for several items at once. Report the selected item IDs, not only the largest score.

### 2. Choice classification

`primary_refactor_kind` returns exactly one label:

```text
page_shape
style_unification
shared_component
composition
state_boundary
i18n
documentation
behavior_drift
mixed
no_action
```

`mixed` means at least two non-trivial categories apply. `no_action` is the explicit clean result.

### 3. Score

`refactor_severity` returns a 0–3 ordered decision:

- `0`: already target-shaped; no refactor work
- `1`: minor issue, later slice is acceptable
- `2`: current diff needs the refactor now
- `3`: release-blocking duplication or behavior drift

A score is not a substitute for the independent booleans. Report both.

## Running the check

For one or two files, use one `judge(state, questions)` call. For 20 or more files, freeze the rubric first, then use one `judge_batch(states, questions)` call for the whole set. Do not loop `judge()` over files.

The question set is the one in `web_refactor.json`; do not rewrite the criteria ad hoc for a single file. If a new rule is needed, update the spec and this skill together, then recalibrate.

For each state:

1. Build complete `file` + exact `diff` + path + context.
2. Send the same fixed questions to jev.
3. Record every boolean, the choice label, score, confidence, and any failed/incomplete state.
4. Read only flagged states manually against the file and implementation contract.
5. Treat jev as evidence, not an automatic verdict: a WARN must be fixed or explicitly rejected in the delivery record.
6. A `FAIL` or a release blocker stops publication until dispositioned.

## Hook placement

The checklist is intentionally:

```yaml
hooks: [pre-push, merge]
sla: l2
fail_severity: WARN
```

Do not add it to pre-commit. A pre-commit run sees incomplete diffs and pays jev cost too often. A pre-push run sees a completed web slice; merge reruns the check as the publication gate.

## Calibration

Before changing a threshold, run the same fixed questions over at least:

- one old flat `tab-page-*` file
- one canonical `tab-page/<tab>.rs` + `components/` file
- one file with a repeated semantic class
- one file with inline Chinese copy
- one structural refactor with changed `aria` / `data-testid`

Record false positives and false negatives. Change criteria first; change thresholds second. Never modify the gate binary or use `--no-verify` to hide a finding.

## Report format

For each flagged file, report:

```text
path:
base -> current:
yes/no findings: [ids]
primary_refactor_kind: label
refactor_severity: 0..3
release_blocker: yes/no
evidence: one concrete file/diff observation
disposition: fixed | accepted with reason | deferred with issue
```

At the top, report totals: states judged, booleans true by ID, choice labels, score distribution, failed states, truncations, and any context file that was missing.
