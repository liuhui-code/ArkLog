export type DeviceLogFindOccurrence = {
  lineIndex: number;
  start: number;
  end: number;
  ordinal: number;
};

export class DeviceLogFind {
  private readonly matches: DeviceLogFindOccurrence[] = [];
  private readonly matchesByLine = new Map<number, DeviceLogFindOccurrence[]>();
  private readonly needle: string;

  constructor(readonly query: string, lines: readonly string[]) {
    this.needle = query.toLocaleLowerCase();
    this.append(lines, 0);
  }

  get occurrences(): readonly DeviceLogFindOccurrence[] {
    return this.matches;
  }

  get count() {
    return this.matches.length;
  }

  occurrencesForLine(lineIndex: number) {
    return this.matchesByLine.get(lineIndex) ?? [];
  }

  append(lines: readonly string[], lineOffset: number) {
    if (this.needle.length === 0) return;
    lines.forEach((line, relativeLineIndex) => {
      const lineIndex = lineOffset + relativeLineIndex;
      const haystack = line.toLocaleLowerCase();
      let offset = 0;
      while (offset <= haystack.length - this.needle.length) {
        const start = haystack.indexOf(this.needle, offset);
        if (start < 0) break;
        const occurrence = {
          lineIndex,
          start,
          end: start + this.query.length,
          ordinal: this.matches.length,
        };
        this.matches.push(occurrence);
        const lineMatches = this.matchesByLine.get(lineIndex) ?? [];
        lineMatches.push(occurrence);
        this.matchesByLine.set(lineIndex, lineMatches);
        offset = start + this.query.length;
      }
    });
  }

  normalize(ordinal: number) {
    if (this.count === 0) return 0;
    return ((ordinal % this.count) + this.count) % this.count;
  }

  move(ordinal: number, direction: 1 | -1) {
    return this.normalize(ordinal + direction);
  }
}
