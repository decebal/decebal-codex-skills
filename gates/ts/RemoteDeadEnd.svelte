<script lang="ts">
  /**
   * Renders the two states that stand in for content: "we could not check" and
   * "we checked, and the answer is nothing".
   *
   * Which affordance appears is decided by the state's own shape, not by a prop
   * on this component — an `unreachable` always shows its retry because the type
   * cannot express one without it, and an `empty` shows a re-check only where the
   * caller was able to supply one. See `remote-state.ts` for why the two are not
   * interchangeable.
   */
  import Button from "../components/buttons/Button.svelte" // your design system's button
  import { cn } from "../utils.js" // your class-merge helper
  import type { RemoteDeadEndState } from "./remote-state.js"

  let {
    state,
    tone = "panel",
    class: className = "",
    testId,
  }: {
    state: RemoteDeadEndState
    /** `panel` fills the content slot it replaces; `inline` sits inside a card. */
    tone?: "panel" | "inline"
    class?: string
    testId?: string
  } = $props()

  let defaultTestId = $derived(state.kind === "unreachable" ? "remote-unreachable" : "remote-empty")
</script>

<div
  class={cn(
    "flex text-muted",
    tone === "panel"
      ? "flex-1 flex-col items-center justify-center gap-sp-4 px-sp-9 py-sp-8 text-center"
      : "flex-col items-start gap-sp-3",
    className
  )}
  data-testid={testId ?? defaultTestId}
  data-remote-state={state.kind}
  role={state.kind === "unreachable" ? "alert" : "status"}
>
  <p class="m-0 max-w-[480px] text-sm leading-relaxed">{state.message}</p>

  {#if state.kind === "empty" && state.actor}
    <p class="m-0 max-w-[480px] text-xs leading-relaxed">{state.actor}</p>
  {/if}

  {#if state.kind === "unreachable" && state.detail}
    <p class="m-0 max-w-[480px] text-xs leading-relaxed opacity-80">{state.detail}</p>
  {/if}

  {#if state.kind === "unreachable"}
    <Button variant="secondary" onclick={() => state.recovery.retry()} data-testid="remote-retry">
      {state.recovery.label ?? "Try again"}
    </Button>
  {:else if state.recheck}
    {@const recheck = state.recheck}
    <Button variant="secondary" onclick={() => recheck.recheck()} data-testid="remote-recheck">
      {recheck.label ?? "Check again"}
    </Button>
  {/if}
</div>
