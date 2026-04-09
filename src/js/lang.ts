/** Language label and flag utilities */

const LANG_LABELS: Record<string, string> = {
  en: 'EN', vi: 'VI', es: 'ES', fr: 'FR', de: 'DE',
  zh: 'ZH', ja: 'JA', ko: 'KO', pt: 'PT', ru: 'RU',
  ar: 'AR', hi: 'HI',
};

const LANG_FLAGS: Record<string, string> = {
  en: '\u{1F1EC}\u{1F1E7}', vi: '\u{1F1FB}\u{1F1F3}', es: '\u{1F1EA}\u{1F1F8}',
  fr: '\u{1F1EB}\u{1F1F7}', de: '\u{1F1E9}\u{1F1EA}', zh: '\u{1F1E8}\u{1F1F3}',
  ja: '\u{1F1EF}\u{1F1F5}', ko: '\u{1F1F0}\u{1F1F7}', pt: '\u{1F1F5}\u{1F1F9}',
  ru: '\u{1F1F7}\u{1F1FA}', ar: '\u{1F1F8}\u{1F1E6}', hi: '\u{1F1EE}\u{1F1F3}',
};

export function getLangLabel(code: string): string {
  return LANG_LABELS[code] ?? code.toUpperCase();
}

export function getLangFlag(code: string): string {
  return LANG_FLAGS[code] ?? '\u{1F310}';
}
