use arklog::serve_mcp;
use serde_json::{json, Value};
use std::io::Cursor;

#[test]
fn stdio_negotiates_mcp_and_advertises_read_only_query_tools() {
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{
            "protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":"tools","method":"tools/list"}),
    ];
    let input = messages
        .iter()
        .map(|v| format!("{v}\n"))
        .collect::<String>();
    let mut output = Vec::new();
    serve_mcp(Cursor::new(input), &mut output, 0).unwrap();
    let responses = String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[0]["result"]["protocolVersion"], "2025-11-25");
    assert_eq!(responses[0]["result"]["capabilities"]["tools"], json!({}));
    assert_eq!(responses[1]["id"], "tools");
    let tools = responses[1]["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 3);
    assert_eq!(tools[2]["name"], "arklog_has_logs");
    assert_eq!(tools[2]["inputSchema"]["required"], json!(["regex"]));
    assert_eq!(
        tools[2]["inputSchema"]["properties"]["after"]["type"],
        "integer"
    );
    assert_eq!(
        tools[2]["outputSchema"]["properties"]["exists"]["type"],
        "boolean"
    );
    assert!(tools[1]["inputSchema"].get("required").is_none());
    for bound in ["start", "end"] {
        assert_eq!(
            tools[1]["inputSchema"]["properties"][bound]["type"],
            json!(["integer", "string", "null"])
        );
        let description = tools[1]["inputSchema"]["properties"][bound]["description"]
            .as_str()
            .unwrap();
        assert!(description.contains("RFC3339"));
        assert!(description.contains("local timezone"));
    }
    assert_eq!(tools[0]["name"], "arklog_session");
    assert_eq!(tools[1]["name"], "arklog_query_logs");
    assert_eq!(tools[1]["annotations"]["readOnlyHint"], true);
    assert!(tools[1]["inputSchema"]["properties"].get("regex").is_some());
}

#[test]
fn mcp_queries_the_running_controller_and_reports_invalid_regex_as_tool_error() {
    use arklog::{ArkLogController, McpRuntime};
    use std::thread;
    use std::time::{Duration, Instant};
    let mut controller = ArkLogController::new("unused-hdc", vec![]).unwrap();
    controller
        .state_mut()
        .append_lines([
            "1793527200.000 1 2 I App before",
            "1793527201.000 1 2 E App ERROR match",
        ])
        .unwrap();
    controller.state_mut().apply_filter("hide-all").unwrap();
    let runtime = McpRuntime::start(0).unwrap();
    assert!(runtime.local_addr().ip().is_loopback());
    let port = runtime.local_addr().port();
    let client = thread::spawn(move || {
        let messages = [
            json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"arklog_session","arguments":{}}}),
            json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"arklog_query_logs","arguments":{
                "start":1793527201000_i64,"regex":"error"}}}),
            json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"arklog_query_logs","arguments":{"regex":"("}}}),
            json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"arklog_has_logs","arguments":{"after":1793527200000_i64,"regex":"error"}}}),
            json!({"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"arklog_has_logs","arguments":{"after":1793527201000_i64,"regex":"error"}}}),
            json!({"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"arklog_has_logs","arguments":{"regex":"error"}}}),
            json!({"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"arklog_has_logs","arguments":{}}}),
            json!({"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"arklog_query_logs","arguments":{"regex":"error"}}}),
            json!({"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"arklog_query_logs","arguments":{"start":"11-01 10:00:00.000"}}}),
            json!({"jsonrpc":"2.0","id":11,"method":"tools/call","params":{"name":"arklog_query_logs","arguments":{
                "start":"2026-11-01T18:00:01+08:00","end":"2026-11-01T10:00:01Z","regex":"error"}}}),
            json!({"jsonrpc":"2.0","id":12,"method":"tools/call","params":{"name":"arklog_query_logs","arguments":{
                "start":"2026-02-30 10:00:00"}}}),
            json!({"jsonrpc":"2.0","id":13,"method":"tools/call","params":{"name":"arklog_query_logs","arguments":{
                "start":"2026-11-01T10:00:01.001Z","end":1793527201000_i64}}}),
        ];
        let input = messages
            .iter()
            .map(|v| format!("{v}\n"))
            .collect::<String>();
        let mut output = Vec::new();
        serve_mcp(Cursor::new(input), &mut output, port).unwrap();
        String::from_utf8(output)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).unwrap())
            .collect::<Vec<_>>()
    });
    let deadline = Instant::now() + Duration::from_secs(5);
    while !client.is_finished() {
        assert!(Instant::now() < deadline, "MCP request blocked");
        runtime.pump(&mut controller);
        thread::sleep(Duration::from_millis(1));
    }
    let responses = client.join().unwrap();
    assert_eq!(responses[1]["result"]["structuredContent"]["raw_count"], 2);
    assert_eq!(responses[2]["result"]["isError"], false, "{}", responses[2]);
    let records = responses[2]["result"]["structuredContent"]["records"]
        .as_array()
        .unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["index"], 1);
    assert_eq!(responses[3]["result"]["isError"], true);
    for (index, expected) in [(4, true), (5, false), (6, true)] {
        assert_eq!(
            responses[index]["result"]["isError"], false,
            "{}",
            responses[index]
        );
        assert_eq!(
            responses[index]["result"]["structuredContent"],
            json!({"exists":expected})
        );
    }
    assert_eq!(responses[7]["result"]["isError"], true);
    assert_eq!(
        responses[8]["result"]["structuredContent"]["records"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(responses[9]["result"]["isError"], true);
    assert_eq!(
        responses[10]["result"]["isError"], false,
        "{}",
        responses[10]
    );
    assert_eq!(
        responses[10]["result"]["structuredContent"]["records"],
        json!([{"index":1,"text":"1793527201.000 1 2 E App ERROR match"}])
    );
    for index in [11, 12] {
        assert_eq!(responses[index]["result"]["isError"], true);
    }
    assert_eq!(controller.state().visible_count(), 0);
    drop(runtime);
    assert!(std::net::TcpStream::connect(("127.0.0.1", port)).is_err());
}

#[test]
fn runtime_waits_for_a_complete_query_frame_before_parsing_time_bounds() {
    use arklog::{ArkLogController, McpRuntime};
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpStream;
    use std::thread;
    use std::time::{Duration, Instant};

    let mut controller = ArkLogController::new("unused-hdc", vec![]).unwrap();
    controller
        .state_mut()
        .append_lines(["0.000 epoch"])
        .unwrap();
    let runtime = McpRuntime::start(0).unwrap();
    let address = runtime.local_addr();
    let client = thread::spawn(move || {
        let mut stream = TcpStream::connect(address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        stream
            .write_all(b"{\"name\":\"arklog_query_logs\",\"arguments\":")
            .unwrap();
        thread::sleep(Duration::from_millis(100));
        stream
            .write_all(b"{\"start\":\"1970-01-01T00:00:00Z\",\"end\":0}}\n")
            .unwrap();
        let mut response = String::new();
        BufReader::new(stream).read_line(&mut response).unwrap();
        serde_json::from_str::<Value>(&response).unwrap()
    });
    let deadline = Instant::now() + Duration::from_secs(5);
    while !client.is_finished() {
        assert!(Instant::now() < deadline, "MCP request blocked");
        runtime.pump(&mut controller);
        thread::sleep(Duration::from_millis(1));
    }
    let response = client.join().unwrap();
    assert_eq!(response["isError"], false, "{response}");
    assert_eq!(
        response["structuredContent"]["records"],
        json!([{"index":0,"text":"0.000 epoch"}])
    );
}

#[test]
fn binary_mcp_mode_uses_only_json_stdout_and_needs_no_terminal_or_hdc() {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let mut child = Command::new(env!("CARGO_BIN_EXE_arklog"))
        .arg("--mcp")
        .env("ARKLOG_HDC_PATH", "does-not-exist")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    writeln!(stdin, "{}", json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1"}}})).unwrap();
    writeln!(
        stdin,
        "{}",
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list"})
    )
    .unwrap();
    drop(stdin);
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    let messages = text
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(messages.len(), 2);
    assert_eq!(
        messages[1]["result"]["tools"][1]["name"],
        "arklog_query_logs"
    );
}

#[test]
fn rejects_invalid_initialize_parameters_and_handles_unavailable_runtime() {
    let input = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","id":2,"method":"initialize","params":{
            "protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}),
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"arklog_session"}}),
        json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"unknown"}}),
    ].iter().map(|v| format!("{v}\n")).collect::<String>();
    let mut output = Vec::new();
    serve_mcp(Cursor::new(input), &mut output, 0).unwrap();
    let responses = String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(responses[0]["error"]["code"], -32602);
    assert_eq!(responses[2]["result"]["isError"], true);
    assert!(responses[2]["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("runtime unavailable"));
    assert_eq!(responses[3]["error"]["code"], -32602);
}

#[test]
fn binary_rejects_unusable_configured_ports() {
    use std::process::{Command, Stdio};
    for port in ["0", "65536", "invalid"] {
        let output = Command::new(env!("CARGO_BIN_EXE_arklog"))
            .arg("--mcp")
            .env("ARKLOG_MCP_PORT", port)
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert!(
            !output.status.success(),
            "port {port} should fail before starting MCP"
        );
    }
}
