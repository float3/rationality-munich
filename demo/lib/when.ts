/**
 * A date from `lib/content.ts` (`2026-10-12`) as words, in Munich time.
 * Noon keeps the day from sliding either way across a timezone boundary.
 */
export function when(date: string, options: Intl.DateTimeFormatOptions) {
  return new Date(`${date}T12:00:00Z`).toLocaleDateString('en-GB', {
    timeZone: 'Europe/Berlin',
    ...options,
  });
}
