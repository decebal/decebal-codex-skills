/**
 * The state of a surface whose content comes from across a boundary the UI does
 * not control — an IPC command, a browser operation, an HTTP fetch, or a port
 * that wraps one.
 *
 * Three facts are kept apart because they are different facts:
 *
 *   - `ready`       — we asked, and here is the answer.
 *   - `empty`       — we asked, and the answer is authoritatively nothing or no.
 *   - `unreachable` — we could not ask, so we do not know what the answer is.
 *
 * Collapsing the last two is the defect this module exists to make
 * unexpressible. A reader shown "no courses are turned on" when the truth is
 * "we could not check" believes a wrong fact about their own data, and has no
 * reason to look again.
 *
 * Two affordances, and they are not interchangeable:
 *
 *   - `unreachable` MUST carry a {@link RemoteRetry}. The fetch failed; the
 *     reader's way out is to run it again. `recovery` is required, so a dead end
 *     cannot be constructed — omission is a compile error, not a review comment.
 *   - `empty` may carry a {@link RemoteRecheck}, and can never carry a retry.
 *     The answer is known; a "Try again" would assert the reader caused the
 *     emptiness or can fix it alone, which is the false affordance
 *     `rules/ui-remote-states.md` forbids. A re-check is honest only where the answer
 *     can change without the reader acting — an admin granting access in another
 *     window, a run arriving in someone's review queue.
 *
 * Recovery re-fetches. It is never `location.reload()`: in a desktop webview
 * that tears the whole app down to fix one panel.
 *
 * Put this in the one package every consumer depends on — the app, the web build
 * and any shared component library — because that is the port boundary in
 * practice. Shared components receive these states; each adapter constructs
 * them, and a component never needs to know which host it is running in to
 * render its own dead end correctly.
 */

/**
 * Labels are a closed set so a retry cannot be dressed as a re-check or the
 * reverse. Copy that has to say more belongs in `message`.
 */
export type RemoteRetryLabel = "Try again" | "Reconnect"
export type RemoteRecheckLabel = "Check again" | "Refresh"

/** Re-runs the fetch that failed. Never a page reload. */
export interface RemoteRetry {
  readonly label?: RemoteRetryLabel
  readonly retry: () => unknown
}

/** Asks again for an answer that someone else can change. Never a page reload. */
export interface RemoteRecheck {
  readonly label?: RemoteRecheckLabel
  readonly recheck: () => unknown
}

/** The fetch is in flight. */
export interface RemoteLoading {
  readonly kind: "loading"
  /** Plain-English progress line, e.g. "Loading your courses…". */
  readonly message?: string
  readonly recovery?: never
  readonly recheck?: never
}

/** We asked and got an answer. */
export interface RemoteReady<T> {
  readonly kind: "ready"
  readonly data: T
  readonly recovery?: never
  readonly recheck?: never
}

/**
 * We could not establish the answer. The recovery action is structural: there is
 * no way to build this variant without one.
 */
export interface RemoteUnreachable {
  readonly kind: "unreachable"
  /** What could not be established, in plain English. Never a raw payload. */
  readonly message: string
  /** The reader's way out. Required. */
  readonly recovery: RemoteRetry
  /** A short human-readable cause. Never a payload, never a stack. */
  readonly detail?: string
  /** Banned here: nobody else is going to make this succeed — the fetch failed. */
  readonly recheck?: never
}

/**
 * We asked, and the answer is nothing or no. Authoritative: the reader is not
 * missing information, they have it.
 */
export interface RemoteAuthoritativeEmpty {
  readonly kind: "empty"
  /** What is known to be absent, in plain English. */
  readonly message: string
  /** Who can change this, named in third person. Omit when nobody can. */
  readonly actor?: string
  /** Allowed only where the answer can change without the reader acting. */
  readonly recheck?: RemoteRecheck
  /** Banned here: a retry implies the reader caused this or can fix it alone. */
  readonly recovery?: never
}

/** The two states that render instead of content. */
export type RemoteDeadEndState = RemoteUnreachable | RemoteAuthoritativeEmpty

/** Every state a remote-dependent surface can be in. */
export type RemoteState<T> = RemoteLoading | RemoteReady<T> | RemoteDeadEndState

export function remoteLoading(message?: string): RemoteLoading {
  return { kind: "loading", message }
}

export function remoteReady<T>(data: T): RemoteReady<T> {
  return { kind: "ready", data }
}

/**
 * Build the "we could not check" state. `retry` is a required argument for the
 * same reason `recovery` is a required field: there is no honest version of this
 * state without one.
 */
export function remoteUnreachable(input: {
  message: string
  retry: () => unknown
  label?: RemoteRetryLabel
  detail?: string
}): RemoteUnreachable {
  return {
    kind: "unreachable",
    message: input.message,
    detail: input.detail,
    recovery: { label: input.label, retry: input.retry },
  }
}

/**
 * Build the "we know, and the answer is nothing" state. It takes no `retry`
 * argument at all — offering one is not a thing the caller can express.
 */
export function remoteEmpty(input: {
  message: string
  actor?: string
  recheck?: () => unknown
  label?: RemoteRecheckLabel
}): RemoteAuthoritativeEmpty {
  return {
    kind: "empty",
    message: input.message,
    actor: input.actor,
    recheck: input.recheck ? { label: input.label, recheck: input.recheck } : undefined,
  }
}

export function isRemoteDeadEnd<T>(state: RemoteState<T>): state is RemoteDeadEndState {
  return state.kind === "unreachable" || state.kind === "empty"
}

export function isRemoteUnreachable<T>(state: RemoteState<T>): state is RemoteUnreachable {
  return state.kind === "unreachable"
}

/**
 * The message a failed fetch should carry when the caller has nothing better.
 * `cause` is squeezed to a single human-readable line; anything that reads like
 * a payload is dropped rather than shown.
 */
export function remoteFailureDetail(cause: unknown): string | undefined {
  const raw = cause instanceof Error ? cause.message : typeof cause === "string" ? cause : ""
  const text = raw.trim()
  if (!text) return undefined
  if (/[{}[\]"]/.test(text)) return undefined
  return text.length > 160 ? undefined : text
}
