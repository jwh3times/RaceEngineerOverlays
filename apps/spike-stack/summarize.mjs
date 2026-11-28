// PROTOTYPE (wayfinder ticket #28): summarize a measure-live.ps1 run.
// node apps/spike-stack/summarize.mjs <results dir>
import { readFileSync, readdirSync, existsSync } from "node:fs";
import { join } from "node:path";

const dir = process.argv[2];
if (!dir) throw new Error("usage: node summarize.mjs <results dir>");

const pct = (sorted, p) => sorted[Math.min(sorted.length - 1, Math.floor((sorted.length - 1) * p))];

function frames(csvPath) {
  const lines = readFileSync(csvPath, "utf8").split(/\r?\n/).filter(Boolean);
  const head = lines[0].split(",");
  const col = (n) => head.indexOf(n);
  const ft = col("FrameTime") >= 0 ? col("FrameTime") : col("MsBetweenPresents");
  const mode = col("PresentMode");
  const gpu = col("GPUBusy") >= 0 ? col("GPUBusy") : col("MsGPUActive");
  if (ft < 0) throw new Error(`no frame-time column in ${csvPath}`);
  const times = [], gpus = [], modes = new Map();
  for (const l of lines.slice(1)) {
    const f = l.split(",");
    const t = Number(f[ft]);
    if (!Number.isFinite(t) || t <= 0) continue;
    times.push(t);
    if (gpu >= 0 && Number.isFinite(Number(f[gpu]))) gpus.push(Number(f[gpu]));
    if (mode >= 0) modes.set(f[mode], (modes.get(f[mode]) ?? 0) + 1);
  }
  const s = [...times].sort((a, b) => a - b);
  const mean = times.reduce((a, b) => a + b, 0) / times.length;
  const g = gpus.length ? gpus.reduce((a, b) => a + b, 0) / gpus.length : NaN;
  const top = [...modes].sort((a, b) => b[1] - a[1]).map(([m, n]) => `${m} ${((100 * n) / times.length).toFixed(0)}%`).join("; ");
  return { n: times.length, fps: 1000 / mean, mean, p50: pct(s, 0.5), p99: pct(s, 0.99), p999: pct(s, 0.999), gpu: g, modes: top };
}

function spike(txtPath) {
  if (!existsSync(txtPath)) return "";
  const t = readFileSync(txtPath, "utf8");
  const cpu = t.match(/cpu \(whole tree\)\s+([\d.]+)%/)?.[1];
  const priv = t.match(/private ([\d.]+) MB \(peak/)?.[1];
  const ws = t.match(/working set ([\d.]+) MB \(peak/)?.[1];
  const procs = t.match(/(\d+) processes\n/)?.[1];
  const fresh = t.match(/tick->drawn\s+mean (\d+) us\s+p50 (\d+)\s+p99 (\d+)/);
  return `cpu ${cpu}% | private ${priv} MB | ws ${ws} MB | procs ${procs} | tick->drawn p50 ${fresh?.[2]} us p99 ${fresh?.[3]} us`;
}

console.log("| config | sim frames | avg fps | frame ms mean | p50 | p99 | p99.9 | sim GPU busy ms | present mode | spike |");
console.log("|---|---|---|---|---|---|---|---|---|---|");
for (const f of readdirSync(dir).filter((f) => f.endsWith(".csv")).sort()) {
  const name = f.replace(/\.csv$/, "");
  const r = frames(join(dir, f));
  console.log(
    `| ${name} | ${r.n} | ${r.fps.toFixed(1)} | ${r.mean.toFixed(2)} | ${r.p50.toFixed(2)} | ${r.p99.toFixed(2)} | ${r.p999.toFixed(2)} | ${Number.isNaN(r.gpu) ? "n/a" : r.gpu.toFixed(2)} | ${r.modes} | ${spike(join(dir, name + ".txt"))} |`,
  );
}
