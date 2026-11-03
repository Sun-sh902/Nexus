import type { Locale } from './preferences';
export const formatTimestamp = (iso: string, locale: Locale) => new Intl.DateTimeFormat(locale, {
  year: 'numeric', month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit', timeZoneName: 'short',
}).format(new Date(iso));
export const formatNumber = (value: number, locale: Locale) => new Intl.NumberFormat(locale).format(value);
export const formatBytes = (bytes: number, locale: Locale) => {
  if (bytes < 1024) return `${formatNumber(bytes, locale)} B`;
  const fmt = new Intl.NumberFormat(locale, { maximumFractionDigits: 1 });
  if (bytes < 1024 * 1024) return `${fmt.format(bytes / 1024)} KB`;
  return `${fmt.format(bytes / (1024 * 1024))} MB`;
};
