<script lang="ts">
  import { onMount } from "svelte";

  let { src }: { src?: string } = $props();

  const W = 64;
  const FADE = 1600;
  const BLOBS = [
    { size: 1.25, speed: 0.000045, orbit: 0.18, phase: 0.0, alpha: 1.0 },
    { size: 1.0, speed: -0.00006, orbit: 0.28, phase: 2.1, alpha: 0.85 },
    { size: 0.85, speed: 0.00008, orbit: 0.34, phase: 4.2, alpha: 0.75 },
    { size: 0.7, speed: -0.0001, orbit: 0.22, phase: 5.3, alpha: 0.65 },
  ];

  let canvas: HTMLCanvasElement;
  let layers: { art: HTMLCanvasElement; born: number }[] = [];
  let tainted = $state(false);

  // 絵を一枚　掌ほどに　畳みおく
  function shrink(img: HTMLImageElement): HTMLCanvasElement {
    const c = document.createElement("canvas");
    c.width = c.height = 96;
    const g = c.getContext("2d")!;
    g.imageSmoothingQuality = "high";
    g.drawImage(img, 0, 0, 96, 96);
    return c;
  }

  $effect(() => {
    if (!src) return;
    const img = new Image();
    img.crossOrigin = "anonymous";
    img.decoding = "async";
    img.onload = () => {
      layers.push({ art: shrink(img), born: performance.now() });
      if (layers.length > 2) layers.shift();
    };
    img.src = src;
  });

  // 縦横に　ならして滲む　色の箱
  function boxBlur(px: Uint8ClampedArray, w: number, h: number, r: number) {
    const tmp = new Uint8ClampedArray(px.length);
    const d = 2 * r + 1;
    const pass = (from: Uint8ClampedArray, to: Uint8ClampedArray, len: number, lines: number, step: number, stride: number) => {
      for (let l = 0; l < lines; l++) {
        const base = l * stride;
        for (let c = 0; c < 3; c++) {
          let sum = 0;
          for (let k = -r; k <= r; k++) sum += from[base + Math.min(len - 1, Math.max(0, k)) * step + c];
          for (let i = 0; i < len; i++) {
            to[base + i * step + c] = sum / d;
            sum += from[base + Math.min(len - 1, i + r + 1) * step + c] - from[base + Math.max(0, i - r) * step + c];
          }
        }
      }
    };
    pass(px, tmp, w, h, 4, w * 4);
    pass(tmp, px, h, w, w * 4, 4);
  }

  // 彩りを　少し濃くして　影に沈む
  function tone(px: Uint8ClampedArray) {
    for (let i = 0; i < px.length; i += 4) {
      const g = 0.3 * px[i] + 0.59 * px[i + 1] + 0.11 * px[i + 2];
      px[i] = (g + (px[i] - g) * 1.45) * 0.74;
      px[i + 1] = (g + (px[i + 1] - g) * 1.45) * 0.74;
      px[i + 2] = (g + (px[i + 2] - g) * 1.45) * 0.74;
      px[i + 3] = 255;
    }
  }

  onMount(() => {
    const ctx = canvas.getContext("2d", { willReadFrequently: true })!;
    const still = matchMedia("(prefers-reduced-motion: reduce)").matches;
    let raf = 0;
    let last = 0;

    const size = () => {
      canvas.width = W;
      canvas.height = Math.max(24, Math.round((W * innerHeight) / Math.max(1, innerWidth)));
    };
    size();
    addEventListener("resize", size);

    // 回りつつ　溶けあう色の　渦を描く
    const draw = (t: number) => {
      raf = requestAnimationFrame(draw);
      if (t - last < 33) return;
      last = t;
      const { width: w, height: h } = canvas;
      ctx.globalAlpha = 1;
      ctx.fillStyle = "#14141a";
      ctx.fillRect(0, 0, w, h);
      const clock = still ? 0 : t;
      for (const layer of layers) {
        const fade = Math.min(1, (t - layer.born) / FADE);
        for (const b of BLOBS) {
          const a = clock * b.speed + b.phase;
          const s = Math.max(w, h) * b.size;
          ctx.save();
          ctx.globalAlpha = fade * b.alpha;
          ctx.translate(w / 2 + Math.cos(a * 0.7) * w * b.orbit, h / 2 + Math.sin(a) * h * b.orbit);
          ctx.rotate(a);
          ctx.drawImage(layer.art, -s / 2, -s / 2, s, s);
          ctx.restore();
        }
      }
      if (tainted) return;
      try {
        const img = ctx.getImageData(0, 0, w, h);
        boxBlur(img.data, w, h, 4);
        boxBlur(img.data, w, h, 3);
        tone(img.data);
        ctx.putImageData(img, 0, 0);
      } catch {
        tainted = true;
      }
    };
    raf = requestAnimationFrame(draw);
    return () => {
      cancelAnimationFrame(raf);
      removeEventListener("resize", size);
    };
  });
</script>

<div class="backdrop" aria-hidden="true">
  <canvas bind:this={canvas} class:tainted></canvas>
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
  canvas {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    transform: scale(1.06);
  }
  canvas.tainted {
    filter: blur(28px) saturate(1.5) brightness(0.75);
  }
  .veil {
    position: absolute;
    inset: 0;
    background:
      radial-gradient(120% 90% at 30% 40%, transparent 0%, rgba(0, 0, 0, 0.18) 70%, rgba(0, 0, 0, 0.36) 100%),
      linear-gradient(180deg, rgba(0, 0, 0, 0.08), rgba(0, 0, 0, 0.22));
  }
  .grain {
    position: absolute;
    inset: 0;
    opacity: 0.07;
    mix-blend-mode: overlay;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='160' height='160'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='.9' numOctaves='2' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E");
  }
</style>
