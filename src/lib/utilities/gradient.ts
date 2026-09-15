/** Deterministic string hash -> a pair of hues, used to render placeholder
 * thumbnail gradients without depending on any network image service
 * (XP FLOW is local-first — see section 2.1). */
function hashString(seed: string): number {
  let hash = 0;
  for (let i = 0; i < seed.length; i++) {
    hash = (hash << 5) - hash + seed.charCodeAt(i);
    hash |= 0;
  }
  return Math.abs(hash);
}

export function seededGradient(seed: string): string {
  const hash = hashString(seed);
  const hueA = hash % 360;
  const hueB = (hueA + 40 + (hash % 60)) % 360;
  return `linear-gradient(135deg, oklch(0.4 0.1 ${hueA}) 0%, oklch(0.22 0.08 ${hueB}) 100%)`;
}
