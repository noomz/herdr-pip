//! Raw JSON client for the herdr socket: one newline-delimited request per connection.

use serde_json::{json, Map, Value};
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;

pub fn call(method: &str, params: Value) -> io::Result<Value> {
    let path = std::env::var("HERDR_SOCKET_PATH")
        .map_err(|_| io::Error::new(io::ErrorKind::NotFound, "HERDR_SOCKET_PATH not set"))?;
    let mut stream = UnixStream::connect(path)?;
    let request = json!({"id": "pip", "method": method, "params": params});
    stream.write_all(format!("{request}\n").as_bytes())?;
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line)?;
    let mut response: Value = serde_json::from_str(&line)?;
    if let Some(error) = response.get("error") {
        let code = error["code"].as_str().unwrap_or("error");
        let message = error["message"].as_str().unwrap_or("");
        return Err(io::Error::other(format!("{method}: {code}: {message}")));
    }
    Ok(response["result"].take())
}

/// A pane the peek can show, with its viewport size in cells.
#[derive(Debug, Clone, PartialEq)]
pub struct Source {
    pub pane_id: String,
    pub name: String,
    pub status: Option<String>,
    pub cols: u16,
    pub rows: u16,
}

/// The parts of `session.snapshot` the plugin reads.
pub struct Snapshot {
    /// Pane surface of the focused tab, the area popup sizes resolve against.
    pub area: (u16, u16),
    pub sources: Vec<Source>,
}

pub fn snapshot() -> io::Result<Snapshot> {
    parse_snapshot(&call("session.snapshot", json!({}))?["snapshot"])
        .ok_or_else(|| io::Error::other("session.snapshot: unexpected shape"))
}

pub fn focus(pane_id: &str) -> io::Result<()> {
    call("pane.focus", json!({"pane_id": pane_id})).map(drop)
}

/// How the peek pane opens.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Placement<'a> {
    /// Session-modal popup, available on every herdr.
    Popup,
    /// Non-modal float (fork herdr), hidden while `hide_while_focused` is the focused pane.
    Float { hide_while_focused: Option<&'a str> },
}

/// Opens the peek as a float, or as a popup when the server rejects floats.
/// `width`/`height` are outer cells (a number) or a percent of the pane surface ("60%").
pub fn open_peek(
    hide_while_focused: Option<&str>,
    width: Value,
    height: Value,
    env: Map<String, Value>,
) -> io::Result<()> {
    let plugin_id = std::env::var("HERDR_PLUGIN_ID")
        .map_err(|_| io::Error::new(io::ErrorKind::NotFound, "HERDR_PLUGIN_ID not set"))?;
    let open = |placement| {
        let params = open_params(&plugin_id, placement, width.clone(), height.clone(), env.clone());
        call("plugin.pane.open", params).map(drop)
    };
    open(Placement::Float { hide_while_focused }).or_else(|_| open(Placement::Popup))
}

fn open_params(
    plugin_id: &str,
    placement: Placement,
    width: Value,
    height: Value,
    env: Map<String, Value>,
) -> Value {
    let mut params = json!({
        "plugin_id": plugin_id,
        "entrypoint": "peek",
        "placement": "popup",
        "width": width,
        "height": height,
        "env": env,
    });
    if let Placement::Float { hide_while_focused } = placement {
        // A float must not send `focus: true`; herdr rejects it with invalid_params.
        params["placement"] = json!("float");
        if let Some(pane_id) = hide_while_focused {
            params["hide_while_focused"] = json!(pane_id);
        }
    }
    params
}

fn parse_snapshot(snap: &Value) -> Option<Snapshot> {
    let focused_tab_id = snap["focused_tab_id"].as_str()?;
    let layouts = snap["layouts"].as_array()?;
    let label = |list: &str, key: &str, id: &str| {
        snap[list]
            .as_array()
            .and_then(|items| items.iter().find(|item| item[key] == id))
            .and_then(|item| item["label"].as_str())
            .unwrap_or(id)
            .to_string()
    };

    let area = layouts
        .iter()
        .find(|layout| layout["tab_id"] == focused_tab_id)
        .map(|layout| rect_size(&layout["area"]))?;

    let sources = snap["panes"]
        .as_array()?
        .iter()
        .filter_map(|pane| {
            let pane_id = pane["pane_id"].as_str()?;
            let rect = layouts.iter().find_map(|layout| {
                layout["panes"]
                    .as_array()?
                    .iter()
                    .find(|p| p["pane_id"] == pane_id)
                    .map(|p| &p["rect"])
            })?;
            let (width, height) = rect_size(rect);
            let rows = pane["scroll"]["viewport_rows"].as_u64().map_or(height, |r| r as u16);
            // Borders take the same cells on each axis; overshooting by a gutter column only
            // adds blank cells to the observed frame.
            let cols = width.saturating_sub(height.saturating_sub(rows));
            let pane_label = pane["label"]
                .as_str()
                .or(pane["agent"].as_str())
                .or(pane["terminal_title_stripped"].as_str().filter(|t| !t.is_empty()))
                .unwrap_or(pane_id);
            let status = pane["agent_status"]
                .as_str()
                .filter(|s| pane["agent"].is_string() && *s != "unknown")
                .map(str::to_string);
            Some(Source {
                pane_id: pane_id.to_string(),
                name: format!(
                    "{}/{}/{}",
                    label("workspaces", "workspace_id", pane["workspace_id"].as_str()?),
                    label("tabs", "tab_id", pane["tab_id"].as_str()?),
                    pane_label
                ),
                status,
                cols: cols.max(1),
                rows: rows.max(1),
            })
        })
        .collect();

    Some(Snapshot { area, sources })
}

fn rect_size(rect: &Value) -> (u16, u16) {
    let get = |key: &str| rect[key].as_u64().unwrap_or(0) as u16;
    (get("width"), get("height"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn float_params_hide_while_source_focused_and_never_send_focus() {
        let env = Map::from_iter([("PIP_SOURCE".to_string(), json!("w1:p1"))]);
        let params = open_params(
            "herdr-pip",
            Placement::Float { hide_while_focused: Some("w1:p1") },
            json!(64),
            json!(34),
            env,
        );
        assert_eq!(params["placement"], "float");
        assert_eq!(params["hide_while_focused"], "w1:p1");
        assert_eq!(params["env"]["PIP_SOURCE"], "w1:p1");
        assert!(params.get("focus").is_none());

        let picker = open_params(
            "herdr-pip",
            Placement::Float { hide_while_focused: None },
            json!("60%"),
            json!("50%"),
            Map::new(),
        );
        assert!(picker.get("hide_while_focused").is_none());
    }

    #[test]
    fn popup_params_carry_no_float_fields() {
        let params = open_params("herdr-pip", Placement::Popup, json!(64), json!(34), Map::new());
        assert_eq!(params["placement"], "popup");
        assert!(params.get("hide_while_focused").is_none());
    }

    #[test]
    fn parses_sources_with_names_sizes_and_agent_status() {
        let snap = json!({
            "focused_pane_id": "w1:p2",
            "focused_tab_id": "w1:t1",
            "workspaces": [{"workspace_id": "w1", "label": "Personal"}],
            "tabs": [{"tab_id": "w1:t1", "label": "dev"}],
            "layouts": [{
                "tab_id": "w1:t1",
                "area": {"x": 0, "y": 0, "width": 252, "height": 65},
                "panes": [
                    {"pane_id": "w1:p1", "rect": {"x": 0, "y": 0, "width": 126, "height": 65}},
                    {"pane_id": "w1:p2", "rect": {"x": 126, "y": 0, "width": 126, "height": 65}}
                ]
            }],
            "panes": [
                {"pane_id": "w1:p1", "workspace_id": "w1", "tab_id": "w1:t1", "label": null,
                 "agent": "claude", "agent_status": "working", "scroll": {"viewport_rows": 63}},
                {"pane_id": "w1:p2", "workspace_id": "w1", "tab_id": "w1:t1", "label": "logs",
                 "agent": null, "agent_status": "unknown", "scroll": {"viewport_rows": 63}}
            ]
        });
        let parsed = parse_snapshot(&snap).unwrap();
        assert_eq!(parsed.area, (252, 65));
        assert_eq!(
            parsed.sources,
            vec![
                Source {
                    pane_id: "w1:p1".into(),
                    name: "Personal/dev/claude".into(),
                    status: Some("working".into()),
                    cols: 124,
                    rows: 63,
                },
                Source {
                    pane_id: "w1:p2".into(),
                    name: "Personal/dev/logs".into(),
                    status: None,
                    cols: 124,
                    rows: 63,
                },
            ]
        );
    }
}
