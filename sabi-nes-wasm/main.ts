import init, { WasmEmulator } from './pkg/sabi_nes_wasm.js';

await init();

const emulator = new WasmEmulator('nes-canvas');
let rafId: number | null = null;

// key code → WasmEmulator numeric code (must match key_code_to_button in Rust)
const KEY_MAP: Record<string, number> = {
  'KeyX':       0, // A
  'KeyZ':       1, // B
  'ShiftLeft':  2, // Select
  'ShiftRight': 2, // Select
  'Enter':      3, // Start
  'ArrowUp':    4,
  'ArrowDown':  5,
  'ArrowLeft':  6,
  'ArrowRight': 7,
};

const romPicker    = document.getElementById('rom-picker')    as HTMLInputElement;
const fullscreenBtn = document.getElementById('fullscreen-btn') as HTMLButtonElement;
const canvas       = document.getElementById('nes-canvas')    as HTMLCanvasElement;
const fpsEl        = document.getElementById('fps')           as HTMLSpanElement;

romPicker.addEventListener('change', async (e) => {
  const file = (e.target as HTMLInputElement).files?.[0];
  if (!file) return;

  const data = new Uint8Array(await file.arrayBuffer());
  try {
    emulator.load_rom(data);
  } catch (err) {
    alert(`Failed to load ROM: ${err}`);
    return;
  }

  if (rafId !== null) cancelAnimationFrame(rafId);

  const FRAME_MS = 1000 / 60;
  let lastTime: number | null = null;
  let fpsCount = 0;
  let fpsWindowStart: number | null = null;

  function loop(timestamp: number): void {
    if (lastTime === null || timestamp - lastTime >= FRAME_MS - 1) {
      lastTime = timestamp;
      try {
        emulator.step_frame();
      } catch (err) {
        rafId = null;
        console.error('Emulator error:', err);
        return;
      }
      fpsCount++;
      if (fpsWindowStart === null) fpsWindowStart = timestamp;
      const elapsed = timestamp - fpsWindowStart;
      if (elapsed >= 500) {
        fpsEl.textContent = `${Math.round(fpsCount * 1000 / elapsed)} FPS`;
        fpsCount = 0;
        fpsWindowStart = timestamp;
      }
    }
    rafId = requestAnimationFrame(loop);
  }
  rafId = requestAnimationFrame(loop);
});

document.addEventListener('keydown', (e) => {
  const key = KEY_MAP[e.code];
  if (key !== undefined) {
    e.preventDefault();
    emulator.key_down(key);
  }
});

document.addEventListener('keyup', (e) => {
  const key = KEY_MAP[e.code];
  if (key !== undefined) emulator.key_up(key);
});

fullscreenBtn.addEventListener('click', () => {
  canvas.requestFullscreen().catch((err: Error) => {
    console.warn('Fullscreen failed:', err);
  });
});
