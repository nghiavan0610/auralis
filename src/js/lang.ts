/** Language label and flag utilities */

export interface Language {
  code: string;
  name: string;
  label: string;
  flag: string;
}

export const SUPPORTED_LANGUAGES: Language[] = [
  { code: 'en', name: 'English', label: 'EN', flag: '\u{1F1EC}\u{1F1E7}' },
  { code: 'vi', name: 'Vietnamese', label: 'VI', flag: '\u{1F1FB}\u{1F1F3}' },
  { code: 'es', name: 'Spanish', label: 'ES', flag: '\u{1F1EA}\u{1F1F8}' },
  { code: 'fr', name: 'French', label: 'FR', flag: '\u{1F1EB}\u{1F1F7}' },
  { code: 'de', name: 'German', label: 'DE', flag: '\u{1F1E9}\u{1F1EA}' },
  { code: 'zh', name: 'Chinese', label: 'ZH', flag: '\u{1F1E8}\u{1F1F3}' },
  { code: 'ja', name: 'Japanese', label: 'JA', flag: '\u{1F1EF}\u{1F1F5}' },
  { code: 'ko', name: 'Korean', label: 'KO', flag: '\u{1F1F0}\u{1F1F7}' },
  { code: 'pt', name: 'Portuguese', label: 'PT', flag: '\u{1F1F5}\u{1F1F9}' },
  { code: 'ru', name: 'Russian', label: 'RU', flag: '\u{1F1F7}\u{1F1FA}' },
  { code: 'ar', name: 'Arabic', label: 'AR', flag: '\u{1F1F8}\u{1F1E6}' },
  { code: 'hi', name: 'Hindi', label: 'HI', flag: '\u{1F1EE}\u{1F1F3}' },
  { code: 'it', name: 'Italian', label: 'IT', flag: '\u{1F1EE}\u{1F1F9}' },
  { code: 'nl', name: 'Dutch', label: 'NL', flag: '\u{1F1F3}\u{1F1F1}' },
  { code: 'pl', name: 'Polish', label: 'PL', flag: '\u{1F1F5}\u{1F1F1}' },
  { code: 'tr', name: 'Turkish', label: 'TR', flag: '\u{1F1F9}\u{1F1F7}' },
  { code: 'th', name: 'Thai', label: 'TH', flag: '\u{1F1F9}\u{1F1ED}' },
  { code: 'id', name: 'Indonesian', label: 'ID', flag: '\u{1F1EE}\u{1F1E9}' },
  { code: 'ms', name: 'Malay', label: 'MS', flag: '\u{1F1F2}\u{1F1FE}' },
  { code: 'uk', name: 'Ukrainian', label: 'UK', flag: '\u{1F1FA}\u{1F1E6}' },
  { code: 'cs', name: 'Czech', label: 'CS', flag: '\u{1F1E8}\u{1F1FC}' },
  { code: 'ro', name: 'Romanian', label: 'RO', flag: '\u{1F1F7}\u{1F1F4}' },
  { code: 'sv', name: 'Swedish', label: 'SV', flag: '\u{1F1F8}\u{1F1EA}' },
  { code: 'da', name: 'Danish', label: 'DA', flag: '\u{1F1E9}\u{1F1F0}' },
  { code: 'no', name: 'Norwegian', label: 'NO', flag: '\u{1F1F3}\u{1F1F4}' },
  { code: 'fi', name: 'Finnish', label: 'FI', flag: '\u{1F1EB}\u{1F1EE}' },
  { code: 'el', name: 'Greek', label: 'EL', flag: '\u{1F1EC}\u{1F1F7}' },
  { code: 'he', name: 'Hebrew', label: 'HE', flag: '\u{1F1EE}\u{1F1F1}' },
  { code: 'bn', name: 'Bengali', label: 'BN', flag: '\u{1F1E7}\u{1F1E9}' },
  { code: 'ta', name: 'Tamil', label: 'TA', flag: '\u{1F1F3}\u{1F1F0}' },
  { code: 'te', name: 'Telugu', label: 'TE', flag: '\u{1F1F3}\u{1F1EE}' },
  { code: 'mr', name: 'Marathi', label: 'MR', flag: '\u{1F1F2}\u{1F1F4}' },
  { code: 'ur', name: 'Urdu', label: 'UR', flag: '\u{1F1F5}\u{1F1F0}' },
  { code: 'fa', name: 'Persian', label: 'FA', flag: '\u{1F1EE}\u{1F1F7}' },
  { code: 'fil', name: 'Filipino', label: 'FIL', flag: '\u{1F1F5}\u{1F1ED}' },
  { code: 'sw', name: 'Swahili', label: 'SW', flag: '\u{1F1F8}\u{1F1F2}' },
  { code: 'hu', name: 'Hungarian', label: 'HU', flag: '\u{1F1ED}\u{1F1FA}' },
];

const LANG_LABELS: Record<string, string> = Object.fromEntries(
  SUPPORTED_LANGUAGES.map(lang => [lang.code, lang.label])
);

const LANG_FLAGS: Record<string, string> = Object.fromEntries(
  SUPPORTED_LANGUAGES.map(lang => [lang.code, lang.flag])
);

export function getLangLabel(code: string): string {
  return LANG_LABELS[code] ?? code.toUpperCase();
}

export function getLangFlag(code: string): string {
  return LANG_FLAGS[code] ?? '\u{1F310}';
}
