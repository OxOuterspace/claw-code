//! `--disallowedTools`: a disallowed tool is neither advertised nor runnable.
//! Its own test binary, because the deny set is process-global.

use std::collections::BTreeSet;

use serde_json::json;
use tools::{
    disallowed_tool_error, disallowed_tools, execute_tool, normalize_disallowed_tools,
    set_disallowed_tools, GlobalToolRegistry,
};

fn names(registry: &GlobalToolRegistry) -> BTreeSet<String> {
    registry
        .definitions(None)
        .into_iter()
        .map(|definition| definition.name)
        .collect()
}

#[test]
fn disallowed_tools_are_hidden_and_refused() {
    let before = names(&GlobalToolRegistry::builtin());
    for decoy in [
        "MCP",
        "ListMcpResources",
        "ReadMcpResource",
        "McpAuth",
        "REPL",
    ] {
        assert!(
            before.contains(decoy),
            "{decoy} is a builtin before the deny set"
        );
    }

    // Case-insensitive builtin names, aliases, and an unknown name in one list.
    let denied = normalize_disallowed_tools(&[
        "mcp,ListMcpResources ReadMcpResource".to_string(),
        "McpAuth".to_string(),
        "repl,read,NoSuchTool".to_string(),
    ]);
    assert!(denied.contains("MCP"));
    assert!(denied.contains("REPL"));
    assert!(
        denied.contains("read_file"),
        "alias resolves to the canonical name"
    );
    assert!(
        denied.contains("NoSuchTool"),
        "an unknown name is kept, not an error"
    );
    set_disallowed_tools(denied.clone());
    assert_eq!(disallowed_tools(), &denied);

    let after = names(&GlobalToolRegistry::builtin());
    for gone in [
        "MCP",
        "ListMcpResources",
        "ReadMcpResource",
        "McpAuth",
        "REPL",
        "read_file",
    ] {
        assert!(!after.contains(gone), "{gone} must not be advertised");
    }
    assert!(after.contains("bash"), "an undenied tool stays advertised");
    assert!(after.contains("Skill"));
    assert_eq!(before.len() - after.len(), 6);

    // Named anyway: refused as a failed result, never run.
    let error = execute_tool("MCP", &json!({"server": "x", "tool": "y"}))
        .expect_err("a disallowed tool must not run");
    assert_eq!(error, disallowed_tool_error("MCP"));
    let error = GlobalToolRegistry::builtin()
        .execute("REPL", &json!({"code": "1", "language": "python"}))
        .expect_err("a disallowed tool must not run through the registry either");
    assert_eq!(error, disallowed_tool_error("REPL"));

    // ToolSearch cannot surface it either.
    let found = GlobalToolRegistry::builtin().search("select:MCP,REPL,Skill", 10, None, None);
    let found = serde_json::to_value(found).expect("search output serialises");
    let matches = found["matches"].as_array().expect("matches array");
    assert!(
        matches.iter().any(|name| name == "Skill"),
        "the select still works"
    );
    assert!(!matches.iter().any(|name| name == "MCP" || name == "REPL"));
}
