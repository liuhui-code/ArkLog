export type DeviceLogDecoration = Readonly<{
  start: number;
  end: number;
  className: string;
}>;

export type DeviceLogHighlightSegment = Readonly<{
  start: number;
  end: number;
  className: string;
}>;

export class DeviceLogHighlighter {
  private readonly decorations: readonly DeviceLogDecoration[];

  constructor(
    private readonly raw: string,
    decorations: readonly DeviceLogDecoration[],
  ) {
    this.decorations = decorations.filter((decoration) => (
      decoration.start >= 0
      && decoration.end <= raw.length
      && decoration.start < decoration.end
    ));
  }

  segments(): readonly DeviceLogHighlightSegment[] {
    const boundaries = new Set([0, this.raw.length]);
    this.decorations.forEach(({ start, end }) => {
      boundaries.add(start);
      boundaries.add(end);
    });
    const positions = [...boundaries].sort((left, right) => left - right);

    return positions.slice(0, -1).map((start, index) => {
      const end = positions[index + 1];
      const classNames = new Set<string>();
      this.decorations
        .filter((decoration) => decoration.start < end && decoration.end > start)
        .forEach((decoration) => {
          decoration.className.split(/\s+/u).filter(Boolean).forEach((name) => classNames.add(name));
        });
      return { start, end, className: [...classNames].join(" ") };
    });
  }
}
