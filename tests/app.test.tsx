import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";
import { App } from "../src/App";
import { FakeArkLogApi } from "./support/fake-arklog-api";

afterEach(cleanup);

describe("ArkLog workbench", () => {
  it("starts the selected device stream and renders incoming HiLog lines", async () => {
    const api = new FakeArkLogApi();
    const user = userEvent.setup();

    render(<App api={api} />);

    expect(await screen.findByRole("option", { name: "USB-01 · online" })).toBeVisible();
    await user.click(screen.getByRole("button", { name: "Start stream" }));
    expect(await screen.findByText("Streaming USB-01")).toBeVisible();

    act(() => api.emit({
      streamId: "stream-1",
      deviceId: "USB-01",
      lines: ["08-28 09:31:02.441 1042 1042 I ArkUI: page mounted"],
    }));

    await waitFor(() => {
      expect(getLogLine("08-28 09:31:02.441 1042 1042 I ArkUI: page mounted")).toBeVisible();
    });
    await user.click(screen.getByRole("button", { name: "Stop stream" }));
    expect(await screen.findByText("Stopped")).toBeVisible();
  });

  it("exposes only live HiLog controls without persisted history or storage", async () => {
    const api = new FakeArkLogApi();

    render(<App api={api} />);

    await screen.findByRole("option", { name: "USB-01 · online" });
    expect(screen.queryByRole("button", { name: "Load history" })).not.toBeInTheDocument();
    expect(screen.queryByRole("region", { name: "Log storage" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Clear history" })).not.toBeInTheDocument();
  });

  it("opens conventional in-view find and highlights literal HiLog matches", async () => {
    const api = new FakeArkLogApi();
    const user = userEvent.setup();

    render(<App api={api} />);

    await screen.findByRole("option", { name: "USB-01 · online" });
    await user.click(screen.getByRole("button", { name: "Start stream" }));
    act(() => api.emit({
      streamId: "stream-1",
      deviceId: "USB-01",
      lines: ["Alpha service ready", "noise", "second alpha event"],
    }));

    fireEvent.keyDown(window, { key: "f", ctrlKey: true });

    const findQuery = screen.getByRole("textbox", { name: "Find in HiLog" });
    expect(findQuery).toHaveFocus();
    await user.type(findQuery, "alpha");

    expect(screen.getByRole("status", { name: "Find results" })).toHaveTextContent("1 / 2");
    const highlights = screen.getAllByText(/alpha/iu, { selector: "mark" });
    expect(highlights).toHaveLength(2);
    expect(highlights[0].closest("li")).toHaveAttribute("aria-current", "true");
  });

  it("navigates find matches in both directions and closes without changing logs", async () => {
    const api = new FakeArkLogApi();
    const user = userEvent.setup();

    render(<App api={api} />);

    await screen.findByRole("option", { name: "USB-01 · online" });
    await user.click(screen.getByRole("button", { name: "Start stream" }));
    act(() => api.emit({
      streamId: "stream-1",
      deviceId: "USB-01",
      lines: ["Alpha first", "Alpha second"],
    }));
    fireEvent.keyDown(window, { key: "f", metaKey: true });

    const findQuery = screen.getByRole("textbox", { name: "Find in HiLog" });
    await user.type(findQuery, "alpha");
    fireEvent.keyDown(findQuery, { key: "Enter" });
    expect(screen.getByRole("status", { name: "Find results" })).toHaveTextContent("2 / 2");
    expect(screen.getAllByText(/alpha/iu, { selector: "mark" })[1].closest("li"))
      .toHaveAttribute("aria-current", "true");

    fireEvent.keyDown(findQuery, { key: "Enter" });
    expect(screen.getByRole("status", { name: "Find results" })).toHaveTextContent("1 / 2");
    fireEvent.keyDown(findQuery, { key: "Enter", shiftKey: true });
    expect(screen.getByRole("status", { name: "Find results" })).toHaveTextContent("2 / 2");

    screen.getByRole("button", { name: "Next match" }).focus();
    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByRole("textbox", { name: "Find in HiLog" })).not.toBeInTheDocument();
    expect(getLogLine("Alpha first")).toBeVisible();
    expect(getLogLine("Alpha second")).toBeVisible();
    expect(screen.queryAllByText(/alpha/iu, { selector: "mark" })).toHaveLength(0);
  });

  it("uses one regular expression for existing and newly received raw logs", async () => {
    const api = new FakeArkLogApi();
    const user = userEvent.setup();

    render(<App api={api} />);

    await screen.findByRole("option", { name: "USB-01 · online" });
    await user.click(screen.getByRole("button", { name: "Start stream" }));
    act(() => api.emit({
      streamId: "stream-1",
      deviceId: "USB-01",
      lines: ["task-12 done", "task pending"],
    }));

    const query = screen.getByRole("textbox", { name: "Filter logs" });
    await user.type(query, String.raw`task-\d+`);

    expect(getLogLine("task-12 done")).toBeVisible();
    expect(queryLogLine("task pending")).not.toBeInTheDocument();
    expect(screen.queryByRole("checkbox", { name: "Regex" })).not.toBeInTheDocument();

    act(() => api.emit({
      streamId: "stream-1",
      deviceId: "USB-01",
      lines: ["task-99 arrived", "task waiting"],
    }));

    expect(getLogLine("task-12 done")).toBeVisible();
    await waitFor(() => expect(getLogLine("task-99 arrived")).toBeVisible());
    expect(queryLogLine("task pending")).not.toBeInTheDocument();
    expect(queryLogLine("task waiting")).not.toBeInTheDocument();
  });

  it("highlights every regular-expression match without changing the raw line", async () => {
    const api = new FakeArkLogApi();
    const user = userEvent.setup();

    render(<App api={api} />);

    await screen.findByRole("option", { name: "USB-01 · online" });
    await user.click(screen.getByRole("button", { name: "Start stream" }));
    const raw = "task-12 completed before task-34";
    act(() => api.emit({
      streamId: "stream-1",
      deviceId: "USB-01",
      lines: [raw, "task pending"],
    }));

    fireEvent.change(screen.getByRole("textbox", { name: "Filter logs" }), {
      target: { value: String.raw`task-\d+` },
    });

    const viewport = screen.getByRole("region", { name: "HiLog output" });
    expect(within(viewport).getAllByText(/^task-/u, { selector: "mark.regex-match" }))
      .toHaveLength(2);
    expect(viewport.querySelector("code")).toHaveTextContent(raw);
    expect(queryLogLine("task pending")).not.toBeInTheDocument();
  });

  it("composes regular-expression and in-view find highlights on overlapping text", async () => {
    const api = new FakeArkLogApi();
    const user = userEvent.setup();

    render(<App api={api} />);

    await screen.findByRole("option", { name: "USB-01 · online" });
    await user.click(screen.getByRole("button", { name: "Start stream" }));
    const raw = "error: disk failed";
    act(() => api.emit({
      streamId: "stream-1",
      deviceId: "USB-01",
      lines: [raw],
    }));
    fireEvent.change(screen.getByRole("textbox", { name: "Filter logs" }), {
      target: { value: String.raw`error:\s+\w+` },
    });
    fireEvent.keyDown(window, { key: "f", ctrlKey: true });
    await user.type(screen.getByRole("textbox", { name: "Find in HiLog" }), "disk");

    const viewport = screen.getByRole("region", { name: "HiLog output" });
    expect(within(viewport).getByText("disk", {
      selector: "mark.regex-match.find-match.is-current",
    })).toBeVisible();
    expect(viewport.querySelector("code")).toHaveTextContent(raw);
  });

  it("changes displayed logs only when the expression changes or Clear logs is used", async () => {
    const api = new FakeArkLogApi();
    const user = userEvent.setup();

    render(<App api={api} />);

    await screen.findByRole("option", { name: "USB-01 · online" });
    await user.click(screen.getByRole("button", { name: "Start stream" }));
    act(() => api.emit({
      streamId: "stream-1",
      deviceId: "USB-01",
      lines: ["keep first", "drop second"],
    }));

    act(() => api.emit({
      streamId: "stream-1",
      deviceId: "USB-01",
      lines: ["new 0", "new 1"],
    }));

    await waitFor(() => expect(getLogLine("keep first")).toBeVisible());
    await user.type(screen.getByRole("textbox", { name: "Filter logs" }), "^new");

    expect(queryLogLine("keep first")).not.toBeInTheDocument();
    expect(getLogLine("new 0")).toBeVisible();

    await user.click(screen.getByRole("button", { name: "Clear logs" }));
    expect(queryLogLine("new 0")).not.toBeInTheDocument();

    act(() => api.emit({
      streamId: "stream-1",
      deviceId: "USB-01",
      lines: ["new after clear", "ignored after clear"],
    }));
    await waitFor(() => expect(getLogLine("new after clear")).toBeVisible());
    expect(queryLogLine("ignored after clear")).not.toBeInTheDocument();
  });

  it("shows invalid regular expressions inline without matching logs", async () => {
    const api = new FakeArkLogApi();
    const user = userEvent.setup();

    render(<App api={api} />);

    await screen.findByRole("option", { name: "USB-01 · online" });
    await user.click(screen.getByRole("button", { name: "Start stream" }));
    act(() => api.emit({
      streamId: "stream-1",
      deviceId: "USB-01",
      lines: ["still buffered"],
    }));

    await user.type(screen.getByRole("textbox", { name: "Filter logs" }), "(");

    expect(screen.getByRole("alert")).toHaveTextContent(/Invalid regular expression/u);
    expect(queryLogLine("still buffered")).not.toBeInTheDocument();
  });

  it("pauses latest-line following while scrolling and returns on demand", async () => {
    const api = new FakeArkLogApi();
    const user = userEvent.setup();

    render(<App api={api} />);

    await screen.findByRole("option", { name: "USB-01 · online" });
    await user.click(screen.getByRole("button", { name: "Start stream" }));
    act(() => api.emit({
      streamId: "stream-1",
      deviceId: "USB-01",
      lines: ["first line", "second line"],
    }));

    const viewport = screen.getByRole("region", { name: "HiLog output" });
    Object.defineProperties(viewport, {
      clientHeight: { configurable: true, value: 200 },
      scrollHeight: { configurable: true, value: 1_000 },
    });
    viewport.scrollTop = 200;
    fireEvent.scroll(viewport);

    expect(screen.getByRole("button", { name: "Back to latest" })).toBeVisible();
    act(() => api.emit({
      streamId: "stream-1",
      deviceId: "USB-01",
      lines: ["third line"],
    }));
    expect(viewport.scrollTop).toBe(200);

    await user.click(screen.getByRole("button", { name: "Back to latest" }));
    expect(viewport.scrollTop).toBe(1_000);
    expect(screen.queryByRole("button", { name: "Back to latest" })).not.toBeInTheDocument();
  });

  it("retains a 10,000-line burst while bounding mounted log rows", async () => {
    const api = new FakeArkLogApi();
    const user = userEvent.setup();

    render(<App api={api} />);

    await screen.findByRole("option", { name: "USB-01 · online" });
    await user.click(screen.getByRole("button", { name: "Start stream" }));
    const lines = Array.from({ length: 10_000 }, (_, index) => `burst line ${index}`);
    lines[3] = "needle in an offscreen line";

    act(() => api.emit({
      streamId: "stream-1",
      deviceId: "USB-01",
      lines,
    }));

    expect(await screen.findByText("10,000")).toBeVisible();
    const viewport = screen.getByRole("region", { name: "HiLog output" });
    expect(viewport.querySelectorAll("li").length).toBeLessThanOrEqual(200);
    expect(getLogLine("burst line 9999")).toBeVisible();

    Object.defineProperties(viewport, {
      clientHeight: { configurable: true, value: 190 },
      scrollHeight: { configurable: true, value: 190_000 },
    });
    viewport.scrollTop = 95_000;
    fireEvent.scroll(viewport);
    expect(getLogLine("burst line 5000")).toBeVisible();

    act(() => api.emit({
      streamId: "stream-1",
      deviceId: "USB-01",
      lines: ["line after paused scroll"],
    }));
    expect(viewport.scrollTop).toBe(95_000);

    fireEvent.keyDown(window, { key: "f", ctrlKey: true });
    await user.type(screen.getByRole("textbox", { name: "Find in HiLog" }), "needle");
    expect(getLogLine("needle in an offscreen line")).toBeVisible();
    expect(viewport.querySelectorAll("li").length).toBeLessThanOrEqual(200);
  }, 30_000);

  it("refreshes fault logs and inspects the selected raw entry", async () => {
    const api = new FakeArkLogApi();
    api.faultLogResult = {
      deviceId: "USB-01",
      entries: [
        {
          id: "fault-1",
          raw: "Reason: JS_ERROR\nSummary: Render failed\nStacktrace:\n  at render (index.ets:4:2)",
        },
        {
          id: "fault-2",
          raw: "Reason: APP_KILLED\nSummary: Process force stopped",
        },
      ],
      command: 'hdc -t USB-01 shell hidumper -s 1201 -a "-p Faultlogger -l -d"',
      stderr: "",
      status: "ready",
      message: "ok",
    };
    const user = userEvent.setup();

    render(<App api={api} />);

    await screen.findByRole("option", { name: "USB-01 · online" });
    await user.click(screen.getByRole("tab", { name: "Fault Log" }));
    await user.click(screen.getByRole("button", { name: "Refresh Fault Logs" }));

    expect(await screen.findByText("at render (index.ets:4:2)")).toBeVisible();
    expect(api.faultLogDevices).toEqual(["USB-01"]);
    await user.click(screen.getByRole("button", { name: /Process force stopped/u }));
    expect(screen.getByLabelText("Fault Log Inspector")).toHaveTextContent("APP_KILLED");
  });

  it("keeps runtime status and log tabs inside the device control row", async () => {
    const api = new FakeArkLogApi();
    api.devices = [];
    render(<App api={api} />);

    const controls = screen.getByRole("region", { name: "Stream controls" });
    expect(getComputedStyle(controls).height).toBe("48px");
    expect(within(controls).queryByRole("heading", { name: "ArkLog" })).not.toBeInTheDocument();
    expect(within(controls).getByRole("tablist", { name: "Log views" })).toBeVisible();
    expect(await within(controls).findByText("No devices")).toBeVisible();
    expect(within(controls).getAllByText("No devices")).toHaveLength(1);
    expect(within(controls).queryByText(/unavailable/iu)).not.toBeInTheDocument();
    expect(screen.queryByRole("banner")).not.toBeInTheDocument();
  });

  it("switches HiLog and Fault Log inside one stable workspace", async () => {
    const api = new FakeArkLogApi();
    const user = userEvent.setup();
    render(<App api={api} />);

    await screen.findByRole("option", { name: "USB-01 · online" });
    const workspace = screen.getByRole("tabpanel", { name: "HiLog" });

    expect(screen.getAllByRole("tabpanel")).toHaveLength(1);
    expect(within(workspace).queryByRole("heading", { name: "HiLog output" })).not.toBeInTheDocument();
    await user.type(within(workspace).getByRole("textbox", { name: "Filter logs" }), "width");

    await user.click(screen.getByRole("tab", { name: "Fault Log" }));

    expect(screen.getAllByRole("tabpanel")).toHaveLength(1);
    expect(screen.getByRole("tabpanel", { name: "Fault Log" })).toBe(workspace);
    expect(within(workspace).getByRole("heading", { name: "Fault Log" })).toBeVisible();

    await user.click(screen.getByRole("tab", { name: "HiLog" }));
    expect(within(workspace).getByRole("textbox", { name: "Filter logs" })).toHaveValue("width");
  });
});

function queryLogLine(raw: string) {
  const viewport = screen.queryByRole("region", { name: "HiLog output" });
  if (!viewport) return null;
  return [...viewport.querySelectorAll("code")]
    .find((element) => element.textContent === raw) ?? null;
}

function getLogLine(raw: string) {
  const line = queryLogLine(raw);
  if (!line) throw new Error(`Expected visible HiLog line: ${raw}`);
  return line;
}
