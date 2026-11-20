import { StyleSheet, Text as RNText, type TextProps } from 'react-native';
import { font } from '../theme';

/**
 * Text in Manrope. Android ignores `fontWeight` for custom fonts, so each weight is its own family:
 * this maps the weights the styles already use (500, 600/700) onto the right file.
 */
export function Text({ style, ...rest }: TextProps) {
  const flat = StyleSheet.flatten(style) ?? {};
  const w = String(flat.fontWeight ?? '400');
  const fontFamily = w === '500' ? font.medium : w === '600' || w === '700' || w === 'bold' ? font.semibold : font.regular;
  // fontWeight is reset so Android does not add fake bold on top of the real weight.
  return <RNText {...rest} style={[flat, { fontFamily, fontWeight: 'normal' }]} />;
}
