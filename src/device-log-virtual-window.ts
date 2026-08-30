export type DeviceLogVirtualRange = Readonly<{
  start: number;
  end: number;
  rowHeight: number;
  totalHeight: number;
}>;

type DeviceLogVirtualWindowInput = Readonly<{
  totalRows: number;
  scrollTop: number;
  viewportHeight: number;
  followingLatest: boolean;
  anchorIndex?: number;
}>;

export class DeviceLogVirtualWindow {
  constructor(
    private readonly rowHeight = 19,
    private readonly overscanRows = 20,
    private readonly maxMountedRows = 200,
  ) {}

  range(input: DeviceLogVirtualWindowInput): DeviceLogVirtualRange {
    const totalRows = Math.max(0, input.totalRows);
    const viewportRows = Math.max(1, Math.ceil(input.viewportHeight / this.rowHeight));
    const mountedRows = Math.min(
      this.maxMountedRows,
      viewportRows + (this.overscanRows * 2),
    );
    let start: number;

    if (input.anchorIndex !== undefined) {
      start = input.anchorIndex - Math.floor(mountedRows / 2);
    } else if (input.followingLatest) {
      start = totalRows - mountedRows;
    } else {
      start = Math.floor(input.scrollTop / this.rowHeight) - this.overscanRows;
    }

    start = Math.max(0, Math.min(start, Math.max(0, totalRows - mountedRows)));
    return {
      start,
      end: Math.min(totalRows, start + mountedRows),
      rowHeight: this.rowHeight,
      totalHeight: totalRows * this.rowHeight,
    };
  }
}
