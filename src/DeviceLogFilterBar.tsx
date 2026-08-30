import type { DeviceLogFilterState } from "./device-log-filter";

type DeviceLogFilterBarProps = {
  state: DeviceLogFilterState;
  error: string | null;
  onChange: (patch: Partial<DeviceLogFilterState>) => void;
  onClearLogs: () => void;
};

export function DeviceLogFilterBar({ state, error, onChange, onClearLogs }: DeviceLogFilterBarProps) {
  return (
    <div className="filter-bar">
      <input
        aria-label="Filter logs"
        className="filter-query"
        type="text"
        value={state.query}
        onChange={(event) => onChange({ query: event.currentTarget.value })}
        placeholder="Regular expression"
        aria-invalid={error ? true : undefined}
      />
      <button
        type="button"
        className="clear-filter-button"
        aria-label="Clear logs"
        onClick={onClearLogs}
      >
        Clear logs
      </button>
      {error ? <span className="filter-error" role="alert">{error}</span> : null}
    </div>
  );
}
