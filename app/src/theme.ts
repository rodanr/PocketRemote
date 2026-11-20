// Design tokens for "Aisle Light" (docs/design-brief.md): near-black ground, one warm amber.
// Amber is for the mark, the lead action and the active state only.
export const color = {
  ground: '#07070a',
  surface: '#1c1914',
  surfacePressed: '#2a251c',
  trackpad: '#0e0d0b',
  outline: '#6b5a3e', // about 3:1: used only on unfilled controls (Live, Close tab), never on filled keys
  hairline: '#2a251c', // quiet edge for large surfaces such as the trackpad
  text: '#e8e2d6',
  textMuted: '#a08a6a', // about 6:1 on the ground
  amber: '#ffb454',
  onAmber: '#1a1208',
  ok: '#5fd38d',
  danger: '#ff6b5a',
} as const;

/** One 8 dp rhythm. */
export const space = { xs: 4, s: 8, m: 16, l: 24 } as const;
export const radius = { s: 12, m: 16, pill: 28 } as const;
/** Minimum touch target (dp). */
export const TARGET = 48;

/** Material 3 type roles, in sp (React Native scales these with the system font size). */
export const type = {
  title: { fontSize: 18, fontWeight: '500' as const, color: color.text },
  body: { fontSize: 15, color: color.text },
  label: { fontSize: 12, fontWeight: '500' as const, letterSpacing: 0.4, color: color.textMuted },
  code: { fontSize: 44, fontWeight: '500' as const, letterSpacing: 2, color: color.amber, fontVariant: ['tabular-nums' as const] },
};
