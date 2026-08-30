import { DeviceLogHighlighter, type DeviceLogDecoration } from "./device-log-highlighter";

type DeviceLogHighlightedTextProps = {
  raw: string;
  decorations: readonly DeviceLogDecoration[];
};

export function DeviceLogHighlightedText({ raw, decorations }: DeviceLogHighlightedTextProps) {
  return new DeviceLogHighlighter(raw, decorations).segments().map((segment) => {
    const text = raw.slice(segment.start, segment.end);
    if (segment.className.length === 0) return text;
    return (
      <mark key={`${segment.start}:${segment.end}`} className={segment.className}>
        {text}
      </mark>
    );
  });
}
