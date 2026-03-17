//! Tool use example using anthropic-rust.
//!
//! Run with:
//! ```bash
//! ANTHROPIC_API_KEY=your-key cargo run --example tools
//! ```

use anthropic_rust::{
    AnthropicClient, ContentBlock, Message, MessageRequest, Role, Tool, ToolResult,
    ToolUse,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load API key from environment
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .expect("ANTHROPIC_API_KEY environment variable not set");

    // Create client
    let client = AnthropicClient::new(api_key)?;

    // Define a weather tool
    let weather_tool = Tool::builder("get_weather", "Get the current weather for a location")
        .string_property("location", "The city and state, e.g. San Francisco, CA", true)
        .enum_property(
            "unit",
            "The temperature unit to use",
            vec!["celsius", "fahrenheit"],
            false,
        )
        .build();

    // Create a request with the tool
    let request = MessageRequest::builder("claude-3-5-sonnet-20241022")
        .system("You are a helpful assistant. Use the get_weather tool when users ask about weather.")
        .user("What's the weather like in Tokyo?")
        .tool(weather_tool)
        .max_tokens(1024)
        .build();

    println!("Sending initial request...\n");

    // First request - Claude will likely request a tool call
    let response = client.messages().create(request).await?;

    println!("Response: {}", response.text());
    println!("Stop reason: {:?}", response.stop_reason);

    // Check if Claude wants to use a tool
    if response.has_tool_use() {
        let tool_uses = response.tool_uses().to_vec();
        println!("\nClaude wants to use {} tool(s):", tool_uses.len());

        // Build the next message with tool results
        let mut messages = vec![
            Message::user("What's the weather like in Tokyo?"),
            Message::with_content(Role::Assistant, response.content.clone()),
        ];

        for tool_use in &tool_uses {
            println!("\n  Tool: {} (ID: {})", tool_use.name, tool_use.id);
            println!("  Input: {}", serde_json::to_string_pretty(&tool_use.input)?);

            // Simulate executing the tool
            let result = execute_weather_tool(tool_use);
            println!("  Result: {}", result.content);

            // Add tool result to messages
            messages.push(Message::with_content(
                Role::User,
                vec![ContentBlock::ToolResult(result)],
            ));
        }

        // Send follow-up request with tool results
        println!("\nSending tool results...\n");

        let follow_up_request = MessageRequest::builder("claude-3-5-sonnet-20241022")
            .system("You are a helpful assistant.")
            .tools(vec![Tool::builder("get_weather", "Get weather")
                .string_property("location", "City name", true)
                .enum_property("unit", "Unit", vec!["celsius", "fahrenheit"], false)
                .build()])
            .message(Role::User, "Continue with the weather information.")
            .max_tokens(1024)
            .build();

        let final_response = client.messages().create(follow_up_request).await?;
        println!("Final response: {}", final_response.text());
    }

    Ok(())
}

/// Simulate executing the weather tool
fn execute_weather_tool(tool_use: &ToolUse) -> ToolResult {
    match tool_use.name.as_str() {
        "get_weather" => {
            let location = tool_use.get_string("location").unwrap_or("Unknown");
            let unit = tool_use.get_string("unit").unwrap_or("celsius");

            // Simulate weather data
            let temp = if unit == "celsius" { "22°C" } else { "72°F" };
            let condition = "Partly cloudy";

            ToolResult::success(
                &tool_use.id,
                format!(
                    "Weather in {}: {}, {}. Humidity: 65%, Wind: 10 km/h.",
                    location, condition, temp
                ),
            )
        }
        _ => ToolResult::error(&tool_use.id, format!("Unknown tool: {}", tool_use.name)),
    }
}
