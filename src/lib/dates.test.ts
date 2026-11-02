import { describe, it, expect } from 'vitest';
import { addDays, localInput } from './dates';
describe('timezone presentation', () => {
  it('converts UTC across calendar boundaries', () =>
    expect(localInput('2026-11-01T23:00:00Z', 'Asia/Kolkata')).toBe(
      '2026-11-02T04:30',
    ));
  it('uses DST offsets in New York', () =>
    expect(localInput('2026-07-01T12:00:00Z', 'America/New_York')).toBe(
      '2026-07-01T08:00',
    ));
  it('preserves empty dates', () => expect(localInput(null, 'UTC')).toBe(''));
  it('crosses year boundary', () =>
    expect(addDays('2026-12-31', 1)).toBe('2027-01-01'));
});
