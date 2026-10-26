<script lang="ts">
  import { onMount } from "svelte";

  let { src }: { src?: string } = $props();

  const W = 64;
  const FADE = 1600;
  const STEP = 50;
  const BLOBS = [
    { size: 1.25, speed: 0.000045, orbit: 0.18, phase: 0.0, alpha: 1.0 },
    { size: 1.0, speed: -0.00006, orbit: 0.28, phase: 2.1, alpha: 0.85 },
    { size: 0.85, speed: 0.00008, orbit: 0.34, phase: 4.2, alpha: 0.75 },
    { size: 0.7, speed: -0.0001, orbit: 0.22, phase: 5.3, alpha: 0.65 },
  ];

  let canvas: HTMLCanvasElement;
  let layers: { art: HTMLCanvasElement; born: number }[] = [];
  let wake = () => {};

  // 絵を畳み　掌ほどの　色となる
  function shrink(img: HTMLImageElement): HTMLCanvasElement {
    const c = document.createElement("canvas");
    c.width = c.height = W;
    const g = c.getContext("2d")!;
    g.imageSmoothingQuality = "high";
    g.drawImage(img, 0, 0, W, W);
    return c;
  }

  // 指の先　霞をひとつ　摘みて置く
  $effect(() => {
    const first = src;
    if (!first) return;
    const img = new Image();
    img.decoding = "async";
    img.onload = () => {
      layers.push({ art: shrink(img), born: performance.now() });
      if (layers.length > 3) layers.shift();
      wake();
    };
    img.src = first;
    return () => {
      img.onload = null;
    };
  });

  onMount(() => {
    const ctx = canvas.getContext("2d")!;
    const work = document.createElement("canvas");
    work.width = work.height = W;
    const wctx = work.getContext("2d")!;
    const still = matchMedia("(prefers-reduced-motion: reduce)").matches;
    let raf = 0;
    let last = 0;

    // 描くたび　霞をまといて　色を置く
    const paint = (t: number) => {
      raf = 0;
      if (document.hidden || !layers.length) return;
      if (t - last < STEP) {
        raf = requestAnimationFrame(paint);
        return;
      }
      last = t;
      const clock = still ? 0 : t;
      wctx.globalAlpha = 1;
      wctx.fillStyle = "#14141a";
      wctx.fillRect(0, 0, W, W);
      for (const layer of layers) {
        const fade = still ? 1 : Math.min(1, (t - layer.born) / FADE);
        for (const b of BLOBS) {
          const a = clock * b.speed + b.phase;
          const s = W * b.size;
          wctx.save();
          wctx.globalAlpha = fade * b.alpha;
          wctx.translate(W / 2 + Math.cos(a * 0.7) * W * b.orbit, W / 2 + Math.sin(a) * W * b.orbit);
          wctx.rotate(a);
          wctx.drawImage(layer.art, -s / 2, -s / 2, s, s);
          wctx.restore();
        }
      }
      // 一枚に　霞と色を　まとわせて
      ctx.clearRect(0, 0, W, W);
      ctx.filter = "blur(3.3px) saturate(1.45) brightness(0.74)";
      ctx.drawImage(work, 0, 0);
      ctx.filter = "none";
      if (!still) raf = requestAnimationFrame(paint);
    };

    wake = () => {
      if (!raf) raf = requestAnimationFrame(paint);
    };
    // 翳りし間　戻りし時こそ　また描け
    const show = () => {
      if (!document.hidden) wake();
    };
    document.addEventListener("visibilitychange", show);
    wake();
    return () => {
      cancelAnimationFrame(raf);
      document.removeEventListener("visibilitychange", show);
      wake = () => {};
    };
  });
</script>

<div class="backdrop" aria-hidden="true">
  <canvas bind:this={canvas} width={W} height={W}></canvas>
  <div class="veil"></div>
  <div class="grain"></div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    overflow: hidden;
    background: #14141a;
    z-index: 0;
  }
  /* 窓変えど　絵は一枚の　まま伸びる */
  canvas {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    transform: scale(1.06);
  }
  .veil {
    position: absolute;
    inset: 0;
    background:
      radial-gradient(120% 90% at 30% 40%, transparent 0%, rgba(0, 0, 0, 0.18) 70%, rgba(0, 0, 0, 0.36) 100%),
      linear-gradient(180deg, rgba(0, 0, 0, 0.08), rgba(0, 0, 0, 0.22));
  }
  /* 砂の粒　絵に宿して　夜を振る */
  .grain {
    position: absolute;
    inset: 0;
    opacity: 1;
    will-change: transform;
    background-image: url("/grain.png");
    background-repeat: repeat;
  }
</style>
