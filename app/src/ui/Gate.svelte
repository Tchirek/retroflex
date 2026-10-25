<script lang="ts">
  import Icon from "./Icon.svelte";

  let { onlink }: { onlink: (text: string) => boolean | Promise<boolean> } = $props();

  let value = $state("");
  let shake = $state(false);

  // 紙切れの　道を貼れば　戸が開く
  async function submit() {
    if (!value.trim()) return;
    if (!(await onlink(value))) {
      shake = true;
      setTimeout(() => (shake = false), 450);
    }
  }
</script>

<div class="gate">
  <img class="mark" src="/icon.svg" alt="" />
  <label class="field glass" class:shake>
    <Icon name="link" size={18} />
    <input
      bind:value
      aria-label="Link"
      spellcheck="false"
      autocomplete="off"
      onkeydown={(e) => e.key === "Enter" && submit()}
      onpaste={() => setTimeout(submit)}
    />
  </label>
</div>

<style>
  .gate {
    position: relative;
    z-index: 1;
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 36px;
    padding: 24px;
  }
  .mark {
    width: 132px;
    height: 132px;
    filter: drop-shadow(0 30px 60px rgba(0, 0, 0, 0.6));
  }
  .field {
    display: flex;
    align-items: center;
    gap: 10px;
    width: min(440px, 100%);
    height: 48px;
    padding: 0 16px;
    border-radius: 14px;
    color: var(--fg-3);
  }
  .field input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: none;
    background: none;
    color: var(--fg);
  }
  .shake {
    animation: shake 0.42s var(--ease);
  }
  @keyframes shake {
    25% {
      transform: translateX(-8px);
    }
    50% {
      transform: translateX(7px);
    }
    75% {
      transform: translateX(-4px);
    }
  }
</style>
