// 一齣ごと　画面の針を　進めけり
export const frame = $state({ t: 0 });

const STEP = 50;
let running = false;

// 刻み足　二十の拍に　息を継ぐ
export function startFrames(): void {
  if (running) return;
  running = true;
  const tick = () => {
    if (!document.hidden) frame.t = performance.timeOrigin + performance.now();
    window.setTimeout(tick, STEP);
  };
  tick();
}
