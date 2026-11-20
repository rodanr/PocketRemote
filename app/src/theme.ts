// Design tokens for "Aisle Light" (docs/design-brief.md): a warm charcoal ground (not pure black), one warm amber.
// Keys are lighter than the ground and carry a 3:1 edge, so they read as buttons (reviewed with Opus).
// Amber is for the mark, the lead action and the active state only.
export const color = {
  ground: '#141210',
  surface: '#2e2a24', // key fill
  surfacePressed: '#3a352d',
  panel: '#1f1c18', // large surfaces: trackpad, dialogs
  rim: '#6b6152', // key edge, about 3:1 against the ground
  rimStrong: '#4a4338',
  groove: '#0c0b09', // recessed areas (the well around OK)
  trackpad: '#1f1c18',
  outline: '#6b5a3e', // about 3:1: used only on unfilled controls (Live, Close tab), never on filled keys
  hairline: '#34302a', // quiet edge for large surfaces such as the trackpad
  text: '#ede7dc',
  textMuted: '#b0a08a', // about 7:1 on the ground
  amber: '#ffb454',
  onAmber: '#1a1208',
  ok: '#5fd38d',
  danger: '#ff7a6a',
} as const;

/** Manrope (free, OFL), loaded in App.tsx. One family per weight, because Android ignores fontWeight for custom fonts. */
export const font = {
  regular: 'Manrope_400Regular',
  medium: 'Manrope_500Medium',
  semibold: 'Manrope_600SemiBold',
} as const;

/** One 8 dp rhythm. */
export const space = { xs: 4, s: 8, m: 16, l: 24 } as const;
export const radius = { s: 12, m: 16, pill: 28 } as const;
/** Minimum touch target (dp). */
export const TARGET = 48;

/** Material 3 type roles, in sp (React Native scales these with the system font size). */
export const type = {
  title: { fontSize: 18, fontWeight: '500' as const, color: color.text },
  body: { fontSize: 15, color: color.text, fontFamily: font.regular },
  label: { fontSize: 12, fontWeight: '500' as const, letterSpacing: 0.4, color: color.textMuted },
  code: { fontSize: 44, fontWeight: '500' as const, letterSpacing: 2, color: color.amber, fontVariant: ['tabular-nums' as const] },
};
