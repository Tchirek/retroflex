// 一齣ごと　画面の針を　進めけり
export const frame = $state({ t: 0 });

const STEP = 16;
let running = false;

// 時を追う　息を弾ませ　針進む
export function startFrames(): void {
  if (running) return;
  running = true;
  const tick = () => {
    if (!document.hidden) frame.t = performance.timeOrigin + performance.now();
    window.setTimeout(tick, STEP);
  };
  tick();
}
