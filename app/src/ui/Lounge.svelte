<script lang="ts">
  import { fade } from "svelte/transition";
  import { backable } from "../lib/back.svelte";
  import { haptic } from "../lib/haptics";
  import Icon from "./Icon.svelte";

  let { onmatch, oncode }: { onmatch: () => Promise<boolean>; oncode: (code: string) => Promise<boolean> } = $props();

  let open = $state(false);
  let busy = $state(false);
  let code = $state("");
  let shake = $state(false);
  let box: HTMLDivElement;

  backable(
    () => open,
    () => (open = false),
  );

  function away(e: PointerEvent) {
    if (open && !box.contains(e.target as Node)) open = false;
  }

  // 広間へと　耳の合う席　探しにゆく
  async function match() {
    if (busy) return;
    busy = true;
    haptic();
    const ok = await onmatch();
    busy = false;
    if (ok) open = false;
  }

  // 六つ字が　揃えば戸口へ　すぐ向かう
  async function typed() {
    code = code.toUpperCase().replace(/[^A-Z0-9]/g, "").slice(0, 6);
    if (code.length < 6) return;
    if (await oncode(code)) {
      open = false;
      code = "";
      return;
    }
    haptic("medium");
    shake = true;
    setTimeout(() => (shake = false), 450);
  }
</script>

<svelte:window onpointerdown={away} />

<div class="lounge" bind:this={box}>
  <button class="icon-btn" class:on={open} aria-label="Lounge" onclick={() => ((open = !open), haptic())}>
    <Icon name="globe" size={19} />
  </button>
  {#if open}
    <!-- 戸を探す　大きな珠と　六つの枡 -->
    <div class="card" transition:fade={{ duration: 140 }}>
      <button class="orb" class:busy aria-label="Find a room" onclick={match}>
        <Icon name="globe" size={30} stroke={1.6} />
      </button>
      <input
        class="code"
        class:shake
        bind:value={code}
        oninput={typed}
        maxlength="8"
        aria-label="Code"
        autocomplete="off"
        autocapitalize="characters"
        spellcheck="false"
        placeholder="······"
      />
    </div>
  {/if}
</div>

<style>
  .lounge {
    position: relative;
    flex: none;
  }
  .card {
    position: absolute;
    right: 0;
    top: calc(100% + 10px);
    z-index: 30;
    width: 212px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 14px;
    padding: 18px 14px 14px;
    border-radius: 18px;
    background: rgba(24, 24, 30, 0.86);
    border: 1px solid rgba(255, 255, 255, 0.08);
    backdrop-filter: blur(30px) saturate(160%);
    box-shadow: 0 18px 50px rgba(0, 0, 0, 0.45);
  }
  .orb {
    display: grid;
    place-items: center;
    width: 76px;
    height: 76px;
    border-radius: 50%;
    color: #111;
    background: #fff;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.35);
    transition: transform 0.25s var(--spring);
  }
  .orb:active {
    transform: scale(0.92);
  }
  .orb.busy :global(svg) {
    animation: spin 1.2s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .code {
    width: 100%;
    height: 42px;
    border: 0;
    border-radius: 12px;
    outline: none;
    text-align: center;
    font-family: ui-monospace, "SF Mono", "Cascadia Mono", Menlo, monospace;
    font-size: 20px;
    font-weight: 700;
    letter-spacing: 0.3em;
    text-transform: uppercase;
    background: rgba(255, 255, 255, 0.08);
  }
  .code::placeholder {
    color: var(--fg-4);
  }
  .code:focus {
    background: rgba(255, 255, 255, 0.13);
  }
  .shake {
    animation: shake 0.42s var(--ease);
  }
  @keyframes shake {
    25% {
      transform: translateX(-7px);
    }
    50% {
      transform: translateX(6px);
    }
    75% {
      transform: translateX(-3px);
    }
  }
</style>
