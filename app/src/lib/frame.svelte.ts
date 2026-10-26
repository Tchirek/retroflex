// 一齣ごと　画面の針を　進めけり
export const frame = $state({ t: 0 });

const STEP = 16;
let running = false;

// 時を追う　息を弾ませ　針進む
export function startFrames(): void {
  if (running) return;
  running = true;
  // 拍に乗り　速き窓には　間を置きて
  let last = 0;
  const tick = (now: number) => {
    requestAnimationFrame(tick);
    if (now - last < STEP - 1) return;
    last = now;
    frame.t = performance.timeOrigin + performance.now();
  };
  requestAnimationFrame(tick);
}
