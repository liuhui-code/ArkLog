import { useEffect, useRef } from "react";

type DeviceLogFindBarProps = {
  query: string;
  current: number;
  total: number;
  focusRequest: number;
  onChange(query: string): void;
  onMove(direction: 1 | -1): void;
  onClose(): void;
};

export function DeviceLogFindBar({
  query,
  current,
  total,
  focusRequest,
  onChange,
  onMove,
  onClose,
}: DeviceLogFindBarProps) {
  const queryRef = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    queryRef.current?.focus();
    queryRef.current?.select();
  }, [focusRequest]);

  function handleKeyDown(event: React.KeyboardEvent<HTMLInputElement>) {
    if (event.key === "Enter") {
      event.preventDefault();
      onMove(event.shiftKey ? -1 : 1);
    } else if (event.key === "Escape") {
      event.preventDefault();
      onClose();
    }
  }

  return (
    <div className="find-bar" role="search" aria-label="Find in HiLog controls">
      <input
        ref={queryRef}
        type="text"
        value={query}
        aria-label="Find in HiLog"
        placeholder="Find"
        onChange={(event) => onChange(event.target.value)}
        onKeyDown={handleKeyDown}
      />
      <span role="status" aria-label="Find results">
        {current} / {total}
      </span>
      <button type="button" aria-label="Previous match" disabled={total === 0} onClick={() => onMove(-1)}>
        ↑
      </button>
      <button type="button" aria-label="Next match" disabled={total === 0} onClick={() => onMove(1)}>
        ↓
      </button>
      <button type="button" aria-label="Close find" onClick={onClose}>
        ×
      </button>
    </div>
  );
}
