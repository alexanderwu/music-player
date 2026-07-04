export function formatDuration(secs: number | null | undefined): string {
  if (secs == null || !isFinite(secs)) return "–:––";
  const s = Math.max(0, Math.round(secs));
  const m = Math.floor(s / 60);
  const r = s % 60;
  return `${m}:${r.toString().padStart(2, "0")}`;
}
