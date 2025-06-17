# Matrixon AI Assistant

This crate provides AI assistant functionality for the Matrixon Matrix Server. It includes features like conversation summarization, user behavior analysis, translation services, and LLM integration.

## Features

- Translation Services
- LLM Integration
- QA Bot
- Room Analysis
- User Behavior Analysis
- Conversation Summarization
- Recommendation Engine

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
matrixon-ai-assistant = { path = "../matrixon-ai-assistant" }
```

## Example

```rust
use matrixon_ai_assistant::prelude::*;

async fn example() -> Result<()> {
    let translation_service = TranslationService::new();
    let llm_service = LlmService::new();
    // ... use the services
    Ok(())
}
```

## License

MIT

## Author

arkSong <arksong2018@gmail.com> 
